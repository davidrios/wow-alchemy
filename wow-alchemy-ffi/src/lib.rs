//! C ABI for wow-alchemy. See `include/wow_alchemy.h` for the contract.
//!
//! Every exported function takes raw pointers from foreign code, so each one
//! requires its pointers to be valid as the header describes.
#![allow(clippy::missing_safety_doc)]

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::{RwLock, RwLockReadGuard};

use wow_alchemy_blp::convert::{blp_to_bc1_mipmaps, blp_to_image, rgba_to_bc1_mipmaps};
use wow_alchemy_blp::{BlpFormat, BlpVersion, load_blp_from_buf};
use wow_alchemy_cdbc::WdbFile;
use wow_alchemy_mpq::PatchChain;

pub mod models;
pub mod terrain;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaStatus {
    Ok = 0,
    NotFound = 1,
    Error = -1,
}

#[repr(C)]
pub struct WaBuffer {
    pub ptr: *mut u8,
    pub len: usize,
}

impl WaBuffer {
    const EMPTY: Self = Self {
        ptr: ptr::null_mut(),
        len: 0,
    };

    fn from_vec(data: Vec<u8>) -> Self {
        let boxed = data.into_boxed_slice();
        let len = boxed.len();
        Self {
            ptr: Box::into_raw(boxed).cast(),
            len,
        }
    }
}

#[repr(C)]
pub struct WaImage {
    pub pixels: WaBuffer,
    pub width: u32,
    pub height: u32,
    pub mip_count: u32,
}

/// How a BLP texture is stored, from its header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaBlpInfo {
    pub width: u32,
    pub height: u32,
    /// 0, 1 or 2: BLP0, BLP1 or BLP2.
    pub version: u32,
    /// One of the `WA_BLP_*` encodings.
    pub encoding: u32,
    /// 1, 3 or 5 for DXT1, DXT3 or DXT5, else 0.
    pub dxt: u32,
    /// Alpha bits per pixel the header gives.
    pub alpha_bits: u32,
    /// Mipmap levels the size and the mipmap flag call for.
    pub mip_count: u32,
}

pub const WA_BLP_JPEG: u32 = 0;
pub const WA_BLP_PALETTE: u32 = 1;
pub const WA_BLP_DXT: u32 = 2;
pub const WA_BLP_BGRA: u32 = 3;

/// Reads share the lock, so threads read at once; only adding archives takes it alone.
pub struct WaMpqChain(RwLock<PatchChain>);

pub struct WaDbc(WdbFile);

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(message: impl ToString) {
    // Interior NULs can't appear in a C string.
    let message = message.to_string().replace('\0', " ");
    LAST_ERROR.with(|slot| *slot.borrow_mut() = CString::new(message).ok());
}

/// Runs `body`, recording its error or panic for `wa_last_error`.
fn guard<T>(fallback: T, body: impl FnOnce() -> Result<T, String>) -> T {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(value)) => value,
        Ok(Err(message)) => {
            set_last_error(message);
            fallback
        }
        Err(_) => {
            set_last_error("wow-alchemy panicked");
            fallback
        }
    }
}

unsafe fn utf8_arg<'a>(value: *const c_char) -> Result<&'a str, String> {
    if value.is_null() {
        return Err("string argument is null".into());
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|e| format!("string argument isn't UTF-8: {e}"))
}

unsafe fn bytes_arg<'a>(data: *const u8, len: usize) -> Result<&'a [u8], String> {
    if data.is_null() {
        return Err("data argument is null".into());
    }
    Ok(unsafe { std::slice::from_raw_parts(data, len) })
}

const POISONED: &str = "MPQ chain is unusable after an earlier panic";

fn read(chain: &WaMpqChain) -> Result<RwLockReadGuard<'_, PatchChain>, String> {
    chain.0.read().map_err(|_| POISONED.to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn wa_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| slot.borrow().as_ref().map_or(ptr::null(), |e| e.as_ptr()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_buffer_free(buffer: WaBuffer) {
    if !buffer.ptr.is_null() {
        let slice = ptr::slice_from_raw_parts_mut(buffer.ptr, buffer.len);
        drop(unsafe { Box::from_raw(slice) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wa_mpq_chain_new() -> *mut WaMpqChain {
    Box::into_raw(Box::new(WaMpqChain(RwLock::new(PatchChain::new()))))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_mpq_chain_free(chain: *mut WaMpqChain) {
    if !chain.is_null() {
        drop(unsafe { Box::from_raw(chain) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_mpq_chain_add(
    chain: *mut WaMpqChain,
    path: *const c_char,
    priority: i32,
) -> WaStatus {
    guard(WaStatus::Error, || {
        let path = unsafe { utf8_arg(path) }?;
        unsafe { &*chain }
            .0
            .write()
            .map_err(|_| POISONED.to_string())?
            .add_archive(path, priority)
            .map_err(|e| format!("{path}: {e}"))?;
        Ok(WaStatus::Ok)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_mpq_chain_contains(
    chain: *mut WaMpqChain,
    file_name: *const c_char,
) -> i32 {
    guard(0, || {
        let file_name = unsafe { utf8_arg(file_name) }?;
        Ok(read(unsafe { &*chain })?.contains_file(file_name) as i32)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_mpq_chain_read(
    chain: *mut WaMpqChain,
    file_name: *const c_char,
    out: *mut WaBuffer,
) -> WaStatus {
    guard(WaStatus::Error, || {
        unsafe { out.write(WaBuffer::EMPTY) };
        let file_name = unsafe { utf8_arg(file_name) }?;
        match read(unsafe { &*chain })?.read_file(file_name) {
            Ok(data) => {
                unsafe { out.write(WaBuffer::from_vec(data)) };
                Ok(WaStatus::Ok)
            }
            Err(wow_alchemy_mpq::Error::FileNotFound(_)) => Ok(WaStatus::NotFound),
            Err(e) => Err(format!("{file_name}: {e}")),
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_mpq_chain_list(chain: *mut WaMpqChain, out: *mut WaBuffer) -> WaStatus {
    guard(WaStatus::Error, || {
        unsafe { out.write(WaBuffer::EMPTY) };
        let entries = read(unsafe { &*chain })?
            .list()
            .map_err(|e| e.to_string())?;
        let names: Vec<_> = entries.into_iter().map(|entry| entry.name).collect();
        unsafe { out.write(WaBuffer::from_vec(names.join("\n").into_bytes())) };
        Ok(WaStatus::Ok)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_blp_decode(
    data: *const u8,
    len: usize,
    mip_level: u32,
    out: *mut WaImage,
) -> WaStatus {
    guard(WaStatus::Error, || {
        unsafe {
            out.write(WaImage {
                pixels: WaBuffer::EMPTY,
                width: 0,
                height: 0,
                mip_count: 0,
            })
        };
        let bytes = unsafe { bytes_arg(data, len) }?;
        let blp = load_blp_from_buf(bytes).map_err(|e| e.to_string())?;
        let mip_count = blp.image_count() as u32;
        if mip_level >= mip_count {
            return Err(format!(
                "mip level {mip_level} is out of range: the texture has {mip_count}"
            ));
        }
        let image = blp_to_image(&blp, mip_level as usize)
            .map_err(|e| e.to_string())?
            .into_rgba8();
        let (width, height) = image.dimensions();
        unsafe {
            out.write(WaImage {
                pixels: WaBuffer::from_vec(image.into_raw()),
                width,
                height,
                mip_count,
            })
        };
        Ok(WaStatus::Ok)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_blp_size(
    data: *const u8,
    len: usize,
    width: *mut u32,
    height: *mut u32,
) -> WaStatus {
    guard(WaStatus::Error, || {
        let bytes = unsafe { bytes_arg(data, len) }?;
        let blp = load_blp_from_buf(bytes).map_err(|e| e.to_string())?;
        unsafe {
            width.write(blp.header.width);
            height.write(blp.header.height);
        }
        Ok(WaStatus::Ok)
    })
}

/// How a BLP texture is stored: its version, encoding and DXT kind, without decoding it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_blp_info(data: *const u8, len: usize, out: *mut WaBlpInfo) -> WaStatus {
    guard(WaStatus::Error, || {
        let bytes = unsafe { bytes_arg(data, len) }?;
        let blp = load_blp_from_buf(bytes).map_err(|e| e.to_string())?;
        let (encoding, dxt) = match blp.format().map_err(|e| e.to_string())? {
            BlpFormat::Jpeg => (WA_BLP_JPEG, 0),
            BlpFormat::Raw1 => (WA_BLP_PALETTE, 0),
            BlpFormat::Raw3 => (WA_BLP_BGRA, 0),
            BlpFormat::Dxt1 => (WA_BLP_DXT, 1),
            BlpFormat::Dxt3 => (WA_BLP_DXT, 3),
            BlpFormat::Dxt5 => (WA_BLP_DXT, 5),
        };
        let header = &blp.header;
        let info = WaBlpInfo {
            width: header.width,
            height: header.height,
            version: match header.version {
                BlpVersion::Blp0 => 0,
                BlpVersion::Blp1 => 1,
                BlpVersion::Blp2 => 2,
            },
            encoding,
            dxt,
            alpha_bits: header.alpha_bits(),
            mip_count: header.mipmap_levels() as u32,
        };
        unsafe { out.write(info) };
        Ok(WaStatus::Ok)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_blp_to_bc1(
    data: *const u8,
    len: usize,
    size: u32,
    out: *mut WaBuffer,
) -> WaStatus {
    guard(WaStatus::Error, || {
        unsafe { out.write(WaBuffer::EMPTY) };
        let bytes = unsafe { bytes_arg(data, len) }?;
        let blp = load_blp_from_buf(bytes).map_err(|e| e.to_string())?;
        let mipmaps = blp_to_bc1_mipmaps(&blp, size).map_err(|e| e.to_string())?;
        unsafe { out.write(WaBuffer::from_vec(mipmaps)) };
        Ok(WaStatus::Ok)
    })
}

/// Converts `width` x `height` RGBA pixels, 4 bytes each, row by row, to a BC1
/// mipmap chain whose first level is `size` square, as `wa_blp_to_bc1` does.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_rgba_to_bc1(
    pixels: *const u8,
    width: u32,
    height: u32,
    size: u32,
    out: *mut WaBuffer,
) -> WaStatus {
    guard(WaStatus::Error, || {
        unsafe { out.write(WaBuffer::EMPTY) };
        let length = (width as usize) * (height as usize) * 4;
        let bytes = unsafe { bytes_arg(pixels, length) }?;
        let mipmaps = rgba_to_bc1_mipmaps(bytes, width, height, size)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "pixels don't fill the image".to_string())?;
        unsafe { out.write(WaBuffer::from_vec(mipmaps)) };
        Ok(WaStatus::Ok)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_dbc_open(data: *const u8, len: usize) -> *mut WaDbc {
    guard(ptr::null_mut(), || {
        let bytes = unsafe { bytes_arg(data, len) }?;
        let file = WdbFile::from_bytes(bytes).map_err(|e| e.to_string())?;
        Ok(Box::into_raw(Box::new(WaDbc(file))))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_dbc_free(dbc: *mut WaDbc) {
    if !dbc.is_null() {
        drop(unsafe { Box::from_raw(dbc) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_dbc_info(
    dbc: *const WaDbc,
    record_count: *mut u32,
    field_count: *mut u32,
    record_size: *mut u32,
) {
    let dbc = &unsafe { &*dbc }.0;
    unsafe {
        record_count.write(dbc.record_count());
        field_count.write(dbc.field_count());
        record_size.write(dbc.record_size());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_dbc_get_u32(
    dbc: *const WaDbc,
    record: u32,
    field: u32,
    out: *mut u32,
) -> WaStatus {
    match unsafe { &*dbc }.0.field_u32(record, field) {
        Some(value) => {
            unsafe { out.write(value) };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_dbc_get_string(
    dbc: *const WaDbc,
    record: u32,
    field: u32,
    out: *mut *const u8,
    out_len: *mut usize,
) -> WaStatus {
    match unsafe { &*dbc }.0.field_string(record, field) {
        Some(text) => {
            unsafe {
                out.write(text.as_ptr());
                out_len.write(text.len());
            }
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_alchemy_mpq::ArchiveBuilder;

    /// Two records of [id, name]. "Forest" shares the tail of "Elwynn Forest".
    fn sample_dbc() -> Vec<u8> {
        let strings = b"\0Elwynn Forest\0";
        let mut data = b"WDBC".to_vec();
        for value in [2u32, 2, 8, strings.len() as u32] {
            data.extend(value.to_le_bytes());
        }
        for value in [12u32, 1, 40, 8] {
            data.extend(value.to_le_bytes());
        }
        data.extend(strings);
        data
    }

    fn string_at(dbc: *const WaDbc, record: u32, field: u32) -> (WaStatus, String) {
        let mut text = ptr::null();
        let mut len = 0;
        let status = unsafe { wa_dbc_get_string(dbc, record, field, &mut text, &mut len) };
        let value = if status == WaStatus::Ok {
            let bytes = unsafe { std::slice::from_raw_parts(text, len) };
            String::from_utf8(bytes.to_vec()).unwrap()
        } else {
            String::new()
        };
        (status, value)
    }

    #[test]
    fn dbc_reads_fields_and_shared_suffix_strings() {
        let bytes = sample_dbc();
        let dbc = unsafe { wa_dbc_open(bytes.as_ptr(), bytes.len()) };
        assert!(!dbc.is_null());

        let mut value = 0;
        assert_eq!(
            unsafe { wa_dbc_get_u32(dbc, 1, 0, &mut value) },
            WaStatus::Ok
        );
        assert_eq!(value, 40);

        assert_eq!(string_at(dbc, 0, 1), (WaStatus::Ok, "Elwynn Forest".into()));
        assert_eq!(string_at(dbc, 1, 1), (WaStatus::Ok, "Forest".into()));

        assert_eq!(
            unsafe { wa_dbc_get_u32(dbc, 2, 0, &mut value) },
            WaStatus::NotFound
        );
        assert_eq!(
            unsafe { wa_dbc_get_u32(dbc, 0, 2, &mut value) },
            WaStatus::NotFound
        );
        unsafe { wa_dbc_free(dbc) };
    }

    #[test]
    fn truncated_dbc_reports_an_error() {
        let bytes = &sample_dbc()[..10];
        let dbc = unsafe { wa_dbc_open(bytes.as_ptr(), bytes.len()) };
        assert!(dbc.is_null());
        assert!(!wa_last_error().is_null());
    }

    #[test]
    fn invalid_blp_reports_an_error() {
        let bytes = b"not a texture";
        let mut image = std::mem::MaybeUninit::<WaImage>::uninit();
        let status = unsafe { wa_blp_decode(bytes.as_ptr(), bytes.len(), 0, image.as_mut_ptr()) };
        assert_eq!(status, WaStatus::Error);
        assert!(!wa_last_error().is_null());
    }

    /// A 4x4 BLP2 holding one DXT5 block, with no mipmaps.
    fn dxt5_blp() -> Vec<u8> {
        let mut data = b"BLP2".to_vec();
        data.extend(1u32.to_le_bytes());
        data.extend([2u8, 8, 7, 0]);
        data.extend(4u32.to_le_bytes());
        data.extend(4u32.to_le_bytes());
        let offset = data.len() as u32 + 128 + 1024;
        let mut offsets = [0u32; 16];
        let mut sizes = [0u32; 16];
        (offsets[0], sizes[0]) = (offset, 16);
        offsets.iter().chain(&sizes).for_each(|v| data.extend(v.to_le_bytes()));
        data.extend([0u8; 1024]);
        data.extend([0u8; 16]);
        data
    }

    #[test]
    fn blp_info_reads_the_encoding() {
        let bytes = dxt5_blp();
        let mut info = std::mem::MaybeUninit::<WaBlpInfo>::uninit();
        let status = unsafe { wa_blp_info(bytes.as_ptr(), bytes.len(), info.as_mut_ptr()) };
        assert_eq!(status, WaStatus::Ok);
        let info = unsafe { info.assume_init() };
        assert_eq!(
            info,
            WaBlpInfo {
                width: 4,
                height: 4,
                version: 2,
                encoding: WA_BLP_DXT,
                dxt: 5,
                alpha_bits: 8,
                mip_count: 1,
            }
        );
    }

    #[test]
    fn higher_priority_archive_wins_and_missing_files_are_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base.mpq");
        let patch = dir.path().join("patch.mpq");
        ArchiveBuilder::new()
            .add_file_data(b"base".to_vec(), "shared.txt")
            .build(&base)
            .unwrap();
        ArchiveBuilder::new()
            .add_file_data(b"patch".to_vec(), "shared.txt")
            .build(&patch)
            .unwrap();

        let chain = wa_mpq_chain_new();
        for (path, priority) in [(&patch, 100), (&base, 0)] {
            let path = CString::new(path.to_str().unwrap()).unwrap();
            assert_eq!(
                unsafe { wa_mpq_chain_add(chain, path.as_ptr(), priority) },
                WaStatus::Ok
            );
        }

        let mut buffer = WaBuffer::EMPTY;
        let status = unsafe { wa_mpq_chain_read(chain, c"shared.txt".as_ptr(), &mut buffer) };
        assert_eq!(status, WaStatus::Ok);
        let data = unsafe { std::slice::from_raw_parts(buffer.ptr, buffer.len) };
        assert_eq!(data, b"patch");
        unsafe { wa_buffer_free(buffer) };

        let mut missing = WaBuffer::EMPTY;
        let status = unsafe { wa_mpq_chain_read(chain, c"missing.txt".as_ptr(), &mut missing) };
        assert_eq!(status, WaStatus::NotFound);
        unsafe { wa_mpq_chain_free(chain) };
    }

    #[test]
    fn handles_can_move_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WaMpqChain>();
        assert_send_sync::<WaDbc>();
    }
}
