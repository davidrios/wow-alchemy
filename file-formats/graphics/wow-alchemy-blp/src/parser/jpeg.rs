use super::bounds::get_bounded_slice;
use super::error::Error;
use super::reader::{ByteReader, Cursor};
use super::types::ParseResult;
use crate::types::jpeg::MAX_JPEG_HEADER;
use crate::types::*;
use log::*;

pub fn parse_jpeg_content<'a, F>(
    blp_header: &BlpHeader,
    mut external_mipmaps: F,
    original_input: &'a [u8],
    input: &'a [u8],
) -> ParseResult<BlpJpeg>
where
    F: FnMut(usize) -> Result<Option<&'a [u8]>, Box<dyn std::error::Error>>,
{
    let mut reader = Cursor::new(input);

    let header_size = reader.read_u32_le()?;
    if header_size as usize > MAX_JPEG_HEADER {
        warn!(
            "JPEG header size {header_size} is greater than {MAX_JPEG_HEADER}, that might cause crashes of some tools.",
        );
    }
    // There is two additional bytes that are not covered by the header size
    let header = reader.read_bytes((header_size + 2) as usize)?;
    let mut images = vec![];

    match blp_header.mipmap_locator {
        MipmapLocator::External => {
            let image0_bytes_opt = external_mipmaps(0).map_err(|e| Error::ExternalMipmap(0, e))?;
            let image0_bytes = image0_bytes_opt.ok_or(Error::MissingImage(0))?;
            images.push(image0_bytes.to_vec());

            if blp_header.has_mipmaps() {
                // funny that there is no hard limit for number of mipmaps
                for i in 1..blp_header.mipmaps_count() + 1 {
                    log::trace!("Parsing mipmap level {}/{}", i, blp_header.mipmaps_count());
                    let image_bytes_opt =
                        external_mipmaps(i).map_err(|e| Error::ExternalMipmap(i, e))?;
                    let image_bytes = image_bytes_opt.ok_or(Error::MissingImage(i))?;
                    images.push(image_bytes.to_vec());
                }
            }
        }
        MipmapLocator::Internal { offsets, sizes } => {
            let mut read_image = |i: usize| -> ParseResult<()> {
                let offset = offsets[i];
                let size = sizes[i];
                let image_bytes = get_bounded_slice(original_input, offset, size, i)?;
                images.push(image_bytes.to_vec());
                Ok(())
            };

            read_image(0)?;
            if blp_header.has_mipmaps() {
                for i in 1..(blp_header.mipmaps_count() + 1).min(16) {
                    read_image(i)?;
                }
            }
        }
    }

    Ok(BlpJpeg { header, images })
}
