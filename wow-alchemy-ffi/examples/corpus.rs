//! Runs every WDT, ADT, WMO, M2, BLP and DBC in a 3.3.5a client through the C ABI and prints a
//! digest of what each call returns, one file per line, so two builds can be compared:
//!
//!     cargo run --release -p wow-alchemy-ffi --example corpus -- <client folder> <out.txt> [ext]
//!
//! Lines are `<file>\t<digest>`, or `<file>\tERR <message>` when the file can't be read.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::io::Write;
use std::path::Path;
use std::ptr;
use std::sync::Mutex;

use wow_alchemy_ffi::models::*;
use wow_alchemy_ffi::terrain::*;
use wow_alchemy_ffi::*;

const LOCALE: &str = "enUS";

struct Digest(u64);

impl Digest {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn f32(&mut self, value: f32) {
        self.u32(value.to_bits());
    }

    fn f32s(&mut self, values: &[f32]) {
        for v in values {
            self.f32(*v);
        }
    }

    fn u32s(&mut self, values: &[u32]) {
        for v in values {
            self.u32(*v);
        }
    }

    /// Plain structs of 4-byte fields, without padding or pointers.
    fn plain<T>(&mut self, value: &T) {
        let bytes =
            unsafe { std::slice::from_raw_parts((value as *const T).cast::<u8>(), size_of::<T>()) };
        self.bytes(bytes);
    }

    fn status(&mut self, status: WaStatus) {
        self.i32(status as i32);
    }
}

fn slice<'a, T>(data: *const T, len: usize) -> &'a [T] {
    if data.is_null() || len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    }
}

fn last_error() -> String {
    let error = wa_last_error();
    if error.is_null() {
        "unknown error".into()
    } else {
        unsafe { CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned()
    }
}

fn archives(client: &Path) -> Vec<String> {
    wow_alchemy_mpq::PatchChain::wotlk_client_archives(client, LOCALE)
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

struct Chain(*mut WaMpqChain);

unsafe impl Send for Chain {}

impl Chain {
    fn open(archives: &[String]) -> Self {
        let chain = wa_mpq_chain_new();
        for (i, path) in archives.iter().enumerate() {
            let path = CString::new(path.as_str()).unwrap();
            let status = unsafe { wa_mpq_chain_add(chain, path.as_ptr(), i as i32) };
            assert_eq!(status, WaStatus::Ok, "{}", last_error());
        }
        Self(chain)
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let name = CString::new(name).unwrap();
        let mut buffer = WaBuffer {
            ptr: ptr::null_mut(),
            len: 0,
        };
        if unsafe { wa_mpq_chain_read(self.0, name.as_ptr(), &mut buffer) } != WaStatus::Ok {
            return None;
        }
        let data = slice(buffer.ptr, buffer.len).to_vec();
        unsafe { wa_buffer_free(buffer) };
        Some(data)
    }

    fn list(&self) -> Vec<String> {
        let mut buffer = WaBuffer {
            ptr: ptr::null_mut(),
            len: 0,
        };
        assert_eq!(
            unsafe { wa_mpq_chain_list(self.0, &mut buffer) },
            WaStatus::Ok
        );
        let text = String::from_utf8_lossy(slice(buffer.ptr, buffer.len)).into_owned();
        unsafe { wa_buffer_free(buffer) };
        text.lines().map(str::to_owned).collect()
    }
}

impl Drop for Chain {
    fn drop(&mut self) {
        unsafe { wa_mpq_chain_free(self.0) };
    }
}

fn mesh(d: &mut Digest, mesh: &WaMesh) {
    let n = mesh.vertex_count as usize;
    d.u32(mesh.vertex_count);
    d.f32s(slice(mesh.positions, n * 3));
    d.f32s(slice(mesh.normals, n * 3));
    d.f32s(slice(mesh.tex_coords, n * 2));
    d.bytes(slice(mesh.colors, n * 4));
    d.u32s(slice(mesh.indices, mesh.index_count as usize));
    for batch in slice(mesh.batches, mesh.batch_count as usize) {
        d.plain(batch);
    }
    d.u32s(slice(
        mesh.collision_indices,
        mesh.collision_index_count as usize,
    ));
}

fn wdt(d: &mut Digest, data: &[u8]) -> Result<u32, String> {
    let mut tiles = vec![0u8; 4096];
    let mut flags = 0;
    let status = unsafe { wa_wdt_read(data.as_ptr(), data.len(), tiles.as_mut_ptr(), &mut flags) };
    if status != WaStatus::Ok {
        return Err(last_error());
    }
    d.bytes(&tiles);
    d.u32(flags);
    let mut name = WaBuffer {
        ptr: ptr::null_mut(),
        len: 0,
    };
    let mut placement = unsafe { std::mem::zeroed::<WaPlacement>() };
    let status = unsafe { wa_wdt_global_wmo(data.as_ptr(), data.len(), &mut name, &mut placement) };
    d.status(status);
    if status == WaStatus::Ok {
        d.bytes(slice(name.ptr, name.len));
        d.plain(&placement);
        unsafe { wa_buffer_free(name) };
    }
    Ok(flags)
}

fn adt(d: &mut Digest, data: &[u8], big_alpha: bool) -> Result<(), String> {
    let adt = unsafe { wa_adt_open(data.as_ptr(), data.len(), big_alpha as i32) };
    if adt.is_null() {
        return Err(last_error());
    }
    let name =
        |f: unsafe extern "C" fn(*const WaAdt, u32, *mut *const u8, *mut usize) -> WaStatus,
         d: &mut Digest,
         index: u32| {
            let mut out = ptr::null();
            let mut len = 0;
            let status = unsafe { f(adt, index, &mut out, &mut len) };
            d.status(status);
            d.bytes(slice(out, len));
            status == WaStatus::Ok
        };
    let count = unsafe { wa_adt_texture_count(adt) };
    d.u32(count);
    for i in 0..count {
        name(wa_adt_texture_name, d, i);
    }
    for i in 0..256 {
        let mut c = unsafe { std::mem::zeroed::<WaTerrainChunk>() };
        let status = unsafe { wa_adt_chunk(adt, i, &mut c) };
        d.status(status);
        if status != WaStatus::Ok {
            continue;
        }
        d.u32s(&[c.ix, c.iy, c.flags, c.area_id, c.holes, c.layer_count]);
        d.f32s(&c.position);
        d.f32s(slice(c.heights, 145));
        d.bytes(slice(c.normals.cast::<u8>(), 145 * 3));
        d.u32s(&c.texture_ids);
        d.u32s(&c.layer_flags);
        d.u32s(&c.effect_ids);
        d.bytes(slice(c.alpha, 3 * 4096));
        d.bytes(slice(c.vertex_colors, 145 * 4));
        d.u64(c.no_effect_doodad);
    }
    for (list, name_fn) in [
        (
            wa_adt_doodads
                as unsafe extern "C" fn(*const WaAdt, *mut *const WaPlacement, *mut usize),
            wa_adt_model_name
                as unsafe extern "C" fn(*const WaAdt, u32, *mut *const u8, *mut usize) -> WaStatus,
        ),
        (wa_adt_wmos, wa_adt_wmo_name),
    ] {
        let mut out = ptr::null();
        let mut len = 0;
        unsafe { list(adt, &mut out, &mut len) };
        d.u64(len as u64);
        for p in slice(out, len) {
            d.plain(p);
        }
        let mut i = 0;
        while name(name_fn, d, i) {
            i += 1;
        }
    }
    let count = unsafe { wa_adt_liquid_count(adt) };
    d.u32(count);
    for i in 0..count {
        let mut l = unsafe { std::mem::zeroed::<WaLiquidLayer>() };
        d.status(unsafe { wa_adt_liquid(adt, i, &mut l) });
        d.u32s(&[l.chunk, l.liquid_type]);
        d.u64(l.cells);
        d.f32s(slice(l.heights, 81));
        d.bytes(slice(l.depths, 81));
    }
    unsafe { wa_adt_free(adt) };
    Ok(())
}

fn wmo_root(d: &mut Digest, data: &[u8]) -> Result<u32, String> {
    let wmo = unsafe { wa_wmo_open(data.as_ptr(), data.len()) };
    if wmo.is_null() {
        return Err(last_error());
    }
    let flags = unsafe { wa_wmo_flags(wmo) };
    d.u32(flags);
    d.u32(unsafe { wa_wmo_group_count(wmo) });
    d.u32(unsafe { wa_wmo_ambient_color(wmo) });
    let mut bounds = [0f32; 6];
    unsafe { wa_wmo_bounds(wmo, bounds.as_mut_ptr()) };
    d.f32s(&bounds);
    for i in 0..unsafe { wa_wmo_material_count(wmo) } {
        let mut m = unsafe { std::mem::zeroed::<WaWmoMaterial>() };
        let mut texture = ptr::null();
        let mut len = 0;
        d.status(unsafe { wa_wmo_material(wmo, i, &mut m, &mut texture, &mut len) });
        d.plain(&m);
        d.bytes(slice(texture, len));
    }
    for i in 0..unsafe { wa_wmo_doodad_set_count(wmo) } {
        let mut s = unsafe { std::mem::zeroed::<WaWmoDoodadSet>() };
        d.status(unsafe { wa_wmo_doodad_set(wmo, i, &mut s) });
        d.plain(&s);
    }
    for i in 0..unsafe { wa_wmo_doodad_count(wmo) } {
        let mut doodad = unsafe { std::mem::zeroed::<WaWmoDoodad>() };
        d.status(unsafe { wa_wmo_doodad(wmo, i, &mut doodad) });
        d.plain(&doodad);
        let mut name = ptr::null();
        let mut len = 0;
        d.status(unsafe { wa_wmo_doodad_name(wmo, i, &mut name, &mut len) });
        d.bytes(slice(name, len));
    }
    let mut vertices = ptr::null();
    let mut count = 0;
    unsafe { wa_wmo_portal_vertices(wmo, &mut vertices, &mut count) };
    d.f32s(slice(vertices, count as usize * 3));
    for i in 0..unsafe { wa_wmo_portal_count(wmo) } {
        let mut p = unsafe { std::mem::zeroed::<WaWmoPortal>() };
        d.status(unsafe { wa_wmo_portal(wmo, i, &mut p) });
        d.plain(&p);
    }
    for i in 0..unsafe { wa_wmo_portal_ref_count(wmo) } {
        let mut r = unsafe { std::mem::zeroed::<WaWmoPortalRef>() };
        d.status(unsafe { wa_wmo_portal_ref(wmo, i, &mut r) });
        d.plain(&r);
    }
    unsafe { wa_wmo_free(wmo) };
    Ok(flags)
}

fn wmo_group(d: &mut Digest, data: &[u8], root_flags: u32) -> Result<(), String> {
    let group = unsafe { wa_wmo_group_open(data.as_ptr(), data.len()) };
    if group.is_null() {
        return Err(last_error());
    }
    let mut m = unsafe { std::mem::zeroed::<WaMesh>() };
    unsafe { wa_wmo_group_mesh(group, &mut m) };
    mesh(d, &m);
    d.u32(unsafe { wa_wmo_group_flags(group) });
    let mut refs = ptr::null();
    let mut count = 0;
    unsafe { wa_wmo_group_doodad_refs(group, &mut refs, &mut count) };
    for r in slice(refs, count as usize) {
        d.bytes(&r.to_le_bytes());
    }
    let mut bounds = [0f32; 6];
    unsafe { wa_wmo_group_bounds(group, bounds.as_mut_ptr()) };
    d.f32s(&bounds);
    let (mut start, mut count) = (0, 0);
    unsafe { wa_wmo_group_portal_refs(group, &mut start, &mut count) };
    d.u32s(&[start, count]);
    d.u32(unsafe { wa_wmo_group_transition_batch_count(group) });
    let mut l = unsafe { std::mem::zeroed::<WaWmoLiquid>() };
    let status = unsafe { wa_wmo_group_liquid(group, root_flags, &mut l) };
    d.status(status);
    if status == WaStatus::Ok {
        d.u32s(&[l.vertices_x, l.vertices_y, l.liquid_type]);
        d.f32s(&l.corner);
        d.f32s(slice(l.heights, (l.vertices_x * l.vertices_y) as usize));
        let cells = (l.vertices_x.saturating_sub(1) * l.vertices_y.saturating_sub(1)) as usize;
        d.bytes(slice(l.cells, cells));
    }
    unsafe { wa_wmo_group_free(group) };
    Ok(())
}

fn m2(d: &mut Digest, data: &[u8], skin: &[u8]) -> Result<(), String> {
    let model = unsafe { wa_m2_open(data.as_ptr(), data.len(), skin.as_ptr(), skin.len()) };
    if model.is_null() {
        return Err(last_error());
    }
    let mut m = unsafe { std::mem::zeroed::<WaMesh>() };
    unsafe { wa_m2_mesh(model, &mut m) };
    mesh(d, &m);
    d.f32(unsafe { wa_m2_bounding_radius(model) });
    let (mut positions, mut position_count, mut indices, mut index_count) =
        (ptr::null(), 0, ptr::null(), 0);
    unsafe {
        wa_m2_collision(
            model,
            &mut positions,
            &mut position_count,
            &mut indices,
            &mut index_count,
        )
    };
    d.f32s(slice(positions, position_count as usize * 3));
    d.u32s(slice(indices, index_count as usize));
    for i in 0..m.batch_count {
        let mut material = unsafe { std::mem::zeroed::<WaM2Material>() };
        d.status(unsafe { wa_m2_material(model, i, &mut material) });
        d.plain(&material);
    }
    let mut i = 0;
    loop {
        let mut name = ptr::null();
        let mut len = 0;
        let status = unsafe { wa_m2_texture_name(model, i, &mut name, &mut len) };
        d.status(status);
        if status != WaStatus::Ok {
            break;
        }
        d.bytes(slice(name, len));
        let (mut kind, mut flags) = (0, 0);
        d.status(unsafe { wa_m2_texture_type(model, i, &mut kind, &mut flags) });
        d.u32s(&[kind, flags]);
        i += 1;
    }
    d.f32s(slice(
        unsafe { wa_m2_tex_coords2(model) },
        m.vertex_count as usize * 2,
    ));
    let (mut weights, mut bone_indices) = (ptr::null(), ptr::null());
    unsafe { wa_m2_vertex_bones(model, &mut weights, &mut bone_indices) };
    d.bytes(slice(weights, m.vertex_count as usize * 4));
    d.bytes(slice(bone_indices, m.vertex_count as usize * 4));
    let mut overrides = ptr::null();
    let mut count = 0;
    d.u32(unsafe { wa_m2_combiner_overrides(model, &mut overrides, &mut count) });
    for o in slice(overrides, count as usize) {
        d.bytes(&o.to_le_bytes());
    }
    d.u32(unsafe { wa_m2_first_sequence_ms(model) });
    let bones = unsafe { wa_m2_bone_count(model) };
    for i in 0..bones {
        let mut b = unsafe { std::mem::zeroed::<WaM2Bone>() };
        d.status(unsafe { wa_m2_bone(model, i, &mut b) });
        d.plain(&b);
    }
    let sequences = unsafe { wa_m2_sequence_count(model) };
    let mut first_local = None;
    for i in 0..sequences {
        let mut s = unsafe { std::mem::zeroed::<WaM2Sequence>() };
        d.status(unsafe { wa_m2_sequence(model, i, &mut s) });
        d.plain(&s);
        if first_local.is_none() && s.flags & 0x20 != 0 {
            first_local = Some(i);
        }
    }
    let mut globals = ptr::null();
    let mut count = 0;
    unsafe { wa_m2_global_sequences(model, &mut globals, &mut count) };
    d.u32s(slice(globals, count as usize));
    for i in 0..unsafe { wa_m2_attachment_count(model) } {
        let mut a = unsafe { std::mem::zeroed::<WaM2Attachment>() };
        d.status(unsafe { wa_m2_attachment(model, i, &mut a) });
        d.plain(&a);
    }
    let events = unsafe { wa_m2_event_count(model) };
    for i in 0..events {
        let mut e = unsafe { std::mem::zeroed::<WaM2Event>() };
        d.status(unsafe { wa_m2_event(model, i, &mut e) });
        d.plain(&e);
    }
    for i in 0..unsafe { wa_m2_emitter_count(model) } {
        let mut e = unsafe { std::mem::zeroed::<WaM2Emitter>() };
        d.status(unsafe { wa_m2_emitter(model, i, &mut e) });
        d.plain(&e);
    }
    let effects = unsafe { wa_m2_effects_open(model) };
    for family in 0..3 {
        for i in 0..unsafe { wa_m2_effects_count(effects, family) } {
            for kind in 0..3 {
                let mut t = unsafe { std::mem::zeroed::<WaM2Track>() };
                let status = unsafe { wa_m2_effects_track(effects, family, i, kind, &mut t) };
                d.status(status);
                if status == WaStatus::Ok {
                    let width = match (family, kind) {
                        (0, 0) | (2, 0) | (2, 2) => 3,
                        (2, 1) => 4,
                        _ => 1,
                    };
                    track(d, &t, width);
                }
            }
        }
    }
    unsafe { wa_m2_effects_free(effects) };
    if let Some(sequence) = first_local {
        let clip = unsafe { wa_m2_clip_open(model, sequence, ptr::null(), 0) };
        d.u32(clip.is_null() as u32);
        if !clip.is_null() {
            for bone in 0..bones {
                for kind in 0..3 {
                    let mut t = unsafe { std::mem::zeroed::<WaM2Track>() };
                    let status = unsafe { wa_m2_clip_track(clip, bone, kind, &mut t) };
                    d.status(status);
                    if status == WaStatus::Ok {
                        track(d, &t, if kind == 1 { 4 } else { 3 });
                    }
                }
            }
            for event in 0..events {
                let mut times = ptr::null();
                let mut count = 0;
                d.status(unsafe { wa_m2_clip_event_times(clip, event, &mut times, &mut count) });
                d.u32s(slice(times, count as usize));
            }
            unsafe { wa_m2_clip_free(clip) };
        }
    }
    unsafe { wa_m2_free(model) };
    Ok(())
}

fn track(d: &mut Digest, t: &WaM2Track, width: usize) {
    d.u32(t.interpolation);
    d.i32(t.global_sequence);
    d.u32s(slice(t.times, t.key_count as usize));
    d.f32s(slice(t.values, t.key_count as usize * width));
}

fn blp(d: &mut Digest, data: &[u8]) -> Result<(), String> {
    let (mut width, mut height) = (0, 0);
    if unsafe { wa_blp_size(data.as_ptr(), data.len(), &mut width, &mut height) } != WaStatus::Ok {
        return Err(last_error());
    }
    d.u32s(&[width, height]);
    let mut image = unsafe { std::mem::zeroed::<WaImage>() };
    let status = unsafe { wa_blp_decode(data.as_ptr(), data.len(), 0, &mut image) };
    d.status(status);
    if status == WaStatus::Ok {
        d.u32s(&[image.width, image.height, image.mip_count]);
        d.bytes(slice(image.pixels.ptr, image.pixels.len));
        unsafe { wa_buffer_free(image.pixels) };
    } else {
        return Err(last_error());
    }
    let mut gpu = unsafe { std::mem::zeroed::<WaGpuTexture>() };
    let status = unsafe { wa_blp_to_gpu(data.as_ptr(), data.len(), &mut gpu) };
    d.status(status);
    if status == WaStatus::Ok {
        d.u32s(&[gpu.format, gpu.width, gpu.height, gpu.mip_count]);
        d.bytes(slice(gpu.pixels.ptr, gpu.pixels.len));
        unsafe { wa_buffer_free(gpu.pixels) };
    }
    if width == height && width.is_power_of_two() {
        let mut out = WaBuffer {
            ptr: ptr::null_mut(),
            len: 0,
        };
        let status = unsafe { wa_blp_to_bc1(data.as_ptr(), data.len(), width.min(256), &mut out) };
        d.status(status);
        if status == WaStatus::Ok {
            d.bytes(slice(out.ptr, out.len));
            unsafe { wa_buffer_free(out) };
        }
    }
    Ok(())
}

fn dbc(d: &mut Digest, data: &[u8]) -> Result<(), String> {
    let dbc = unsafe { wa_dbc_open(data.as_ptr(), data.len()) };
    if dbc.is_null() {
        return Err(last_error());
    }
    let (mut records, mut fields, mut size) = (0, 0, 0);
    unsafe { wa_dbc_info(dbc, &mut records, &mut fields, &mut size) };
    d.u32s(&[records, fields, size]);
    for r in 0..=records {
        for f in 0..=fields {
            let mut value = 0;
            let status = unsafe { wa_dbc_get_u32(dbc, r, f, &mut value) };
            d.status(status);
            d.u32(value);
            let mut text = ptr::null();
            let mut len = 0;
            let status = unsafe { wa_dbc_get_string(dbc, r, f, &mut text, &mut len) };
            d.status(status);
            d.bytes(slice(text, len));
        }
    }
    unsafe { wa_dbc_free(dbc) };
    Ok(())
}

fn is_group(lower: &str) -> bool {
    let stem = &lower[..lower.len() - 4];
    stem.len() > 4
        && stem.as_bytes()[stem.len() - 4] == b'_'
        && stem[stem.len() - 3..].bytes().all(|b| b.is_ascii_digit())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: corpus <client folder> <out.txt> [extension]");
        std::process::exit(2);
    }
    let archives = archives(Path::new(&args[1]));
    let only = args.get(3).map(|e| e.to_lowercase());
    let chain = Chain::open(&archives);
    let mut names: Vec<String> = chain
        .list()
        .into_iter()
        .filter(|n| {
            let l = n.to_lowercase();
            let ext = l.rsplit('.').next().unwrap_or("").to_owned();
            ["wdt", "adt", "wmo", "m2", "blp", "dbc"].contains(&ext.as_str())
                && only.as_ref().is_none_or(|o| *o == ext)
        })
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup_by_key(|n| n.to_lowercase());
    drop(chain);
    eprintln!("{} files", names.len());

    let big_alpha: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
    let root_flags: Mutex<HashMap<String, u32>> = Mutex::new(HashMap::new());
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(12);
    let results: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
    let next = Mutex::new(0usize);

    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                let chain = Chain::open(&archives);
                loop {
                    let index = {
                        let mut next = next.lock().unwrap();
                        let i = *next;
                        *next += 1;
                        i
                    };
                    let Some(name) = names.get(index) else { break };
                    if index % 2000 == 0 {
                        eprintln!("{index}/{}", names.len());
                    }
                    let lower = name.to_lowercase();
                    let mut d = Digest::new();
                    let result = match chain.read(name) {
                        None => Err("not readable".to_string()),
                        Some(data) => {
                            let ext = lower.rsplit('.').next().unwrap();
                            match ext {
                                "wdt" => wdt(&mut d, &data).map(|_| ()),
                                "adt" => {
                                    let map = name[..name.rfind('_').unwrap_or(0)]
                                        .rsplit_once('_')
                                        .map_or(String::new(), |(m, _)| m.to_owned());
                                    let big = *big_alpha
                                        .lock()
                                        .unwrap()
                                        .entry(map.clone())
                                        .or_insert_with(|| {
                                            chain
                                                .read(&format!("{map}.wdt"))
                                                .and_then(|w| wdt(&mut Digest::new(), &w).ok())
                                                .is_some_and(|f| f & 4 != 0)
                                        });
                                    adt(&mut d, &data, big)
                                }
                                "wmo" if is_group(&lower) => {
                                    let root = format!("{}.wmo", &name[..name.len() - 8]);
                                    let flags = *root_flags
                                        .lock()
                                        .unwrap()
                                        .entry(root.clone())
                                        .or_insert_with(|| {
                                            chain
                                                .read(&root)
                                                .and_then(|w| wmo_root(&mut Digest::new(), &w).ok())
                                                .unwrap_or(0)
                                        });
                                    wmo_group(&mut d, &data, flags)
                                }
                                "wmo" => wmo_root(&mut d, &data).map(|_| ()),
                                "m2" => {
                                    let skin = format!("{}00.skin", &name[..name.len() - 3]);
                                    match chain.read(&skin) {
                                        Some(skin) => m2(&mut d, &data, &skin),
                                        None => Err("no skin".into()),
                                    }
                                }
                                "blp" => blp(&mut d, &data),
                                "dbc" => dbc(&mut d, &data),
                                _ => unreachable!(),
                            }
                        }
                    };
                    let line = match result {
                        Ok(()) => format!("{:016x}", d.0),
                        Err(e) => format!("ERR {}", e.replace(['\n', '\t'], " ")),
                    };
                    results.lock().unwrap().push((lower, line));
                }
            });
        }
    });

    let mut results = results.into_inner().unwrap();
    results.sort();
    let errors = results.iter().filter(|(_, l)| l.starts_with("ERR")).count();
    let mut out = std::fs::File::create(&args[2]).unwrap();
    for (name, line) in &results {
        writeln!(out, "{name}\t{line}").unwrap();
    }
    eprintln!("{} files, {errors} errors", results.len());
}
