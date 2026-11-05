//! The files in `test-data`, made from its PNGs by the image-blp crate. Every one of their 16
//! mipmap entries points at the same bytes, so they don't write back as they were.

use wow_alchemy_blp::convert::blp_to_image;
use wow_alchemy_blp::{BlpFormat, load_blp};

/// `max_error` is how far each of red, green and blue may be off; alpha is checked when the
/// file has it, except for JPEG, whose fourth channel the `image` crate drops.
fn check(name: &str, format: BlpFormat, png: &str, max_error: u8) {
    let dir = format!("{}/test-data", env!("CARGO_MANIFEST_DIR"));
    let blp = load_blp(format!("{dir}/{name}")).unwrap();
    assert_eq!(blp.format().unwrap(), format, "{name}");
    assert_eq!(blp.image_count(), blp.header.mipmap_levels(), "{name}");

    let source = image::open(format!("{dir}/{png}")).unwrap().into_rgba8();
    let decoded = blp_to_image(&blp, 0).unwrap().into_rgba8();
    assert_eq!(decoded.dimensions(), source.dimensions(), "{name}");
    let difference = |channel: usize| {
        decoded
            .pixels()
            .zip(source.pixels())
            .map(|(a, b)| a[channel].abs_diff(b[channel]))
            .max()
            .unwrap()
    };
    for channel in 0..3 {
        assert!(
            difference(channel) <= max_error,
            "{name}: {}",
            difference(channel)
        );
    }
    if format == BlpFormat::Raw1 && blp.header.alpha_bits() == 8 {
        assert_eq!(difference(3), 0, "{name}");
    }
}

#[test]
fn palettized() {
    check(
        "test_simple_with_alpha.blp",
        BlpFormat::Raw1,
        "test_image_source.png",
        0,
    );
    check(
        "test_simple_without_alpha.blp",
        BlpFormat::Raw1,
        "test_image_source.png",
        0,
    );
    check(
        "test_rect_with_alpha.blp",
        BlpFormat::Raw1,
        "test_image_source_rect.png",
        0,
    );
}

#[test]
fn jpeg() {
    check(
        "test_simple_jpg.blp",
        BlpFormat::Jpeg,
        "test_image_source.png",
        16,
    );
    check(
        "test_rect_without_alpha.blp",
        BlpFormat::Jpeg,
        "test_image_source_rect_no_alpha.png",
        32,
    );
}
