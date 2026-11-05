# wow-alchemy-blp

Reader and writer for BLP textures (BLP0 and BLP1 of Warcraft III, BLP2 of World of Warcraft),
and conversion between them and images.

- `load_blp_from_buf`, `load_blp` (which also reads BLP0's `.b00`, `.b01`... mipmap files) and
  `save_blp` read and write `BlpImage`: the header, the palette or JPEG header, and each
  mipmap's bytes as stored. Writing a file read gives the same bytes back.
- `BlpImage::format` says what the mipmaps hold: JPEG, palettized (Raw1), BGRA (Raw3) or DXT1,
  DXT3 or DXT5.
- `convert::blp_to_image` decodes a level, `convert::image_to_blp` encodes an image as
  palettized, BGRA or DXT, and `convert::blp_to_bc1_mipmaps` makes a BC1 mipmap chain for a
  GPU. JPEG content is read but not written.

## License

Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
<http://www.apache.org/licenses/LICENSE-2.0>)

## Credits

Based on the [image-blp](https://github.com/zloy-tulen/image-blp) crate by zloy_tulen.
