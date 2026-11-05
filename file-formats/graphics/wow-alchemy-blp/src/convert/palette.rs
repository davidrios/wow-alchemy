use ::image::RgbaImage;
use color_quant::NeuQuant;

/// Quantizes to 255 colors, giving the indices, the colors as palette entries and the
/// quantizer, for the mipmaps.
pub fn quantize_rgba(img: RgbaImage) -> (Vec<u8>, Vec<u32>, NeuQuant) {
    // The last of the file's 256 colors isn't used.
    let palette_size = 255;
    // 1 is the best quality, 30 the fastest.
    let sample_fact = 10;

    let img = without_alpha(img);
    let nq = NeuQuant::new(sample_fact, palette_size, img.as_raw());
    let indices = index(&img, &nq);
    let palette = nq
        .color_map_rgb()
        .chunks(3)
        // Stored BGRA, so read as a little-endian u32, red is the third byte.
        .map(|col| ((col[0] as u32) << 16) | ((col[1] as u32) << 8) | col[2] as u32)
        .collect();
    (indices, palette, nq)
}

pub fn quantize_rgba_known(img: RgbaImage, nq: &NeuQuant) -> Vec<u8> {
    index(&without_alpha(img), nq)
}

/// Alpha is stored apart, so it mustn't sway the colors.
fn without_alpha(mut img: RgbaImage) -> RgbaImage {
    for pixel in img.pixels_mut() {
        pixel[3] = 0;
    }
    img
}

fn index(img: &RgbaImage, nq: &NeuQuant) -> Vec<u8> {
    img.pixels()
        .map(|pixel| nq.index_of(&pixel.0) as u8)
        .collect()
}
