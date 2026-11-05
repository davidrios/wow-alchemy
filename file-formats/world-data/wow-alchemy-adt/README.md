# wow-alchemy-adt

Reader and writer for World of Warcraft ADT terrain tiles, as stored up to WotLK (version 18,
one file per tile). A tile read and written back is the same bytes: every ADT of a 3.3.5a client
round-trips.

- `Adt`: the file, read with `WowStructR::wow_read` and written with `WowStructW::wow_write`:
  MHDR, the texture, model and WMO names (MTEX, MMDX/MMID, MWMO/MWID), placements (MDDF, MODF),
  MH2O, the 256 MCNKs, MFBO and MTXF. Unknown chunks are kept.
- `McnkChunk`: a map chunk's header and its MCVT heights, MCCV colors, MCNR normals, MCLY layers,
  MCRF references, MCSH shadows, MCAL alpha maps, MCLQ liquids and MCSE sound emitters.
- `McnkChunk::alpha_map` / `decode_alpha_map`: alpha maps decoded to 64 by 64 bytes, from 4-bit
  (with the client's 63 by 63 fix unless the chunk says not to), 8-bit or compressed data.
- `Mh2o::liquid_layers`: MH2O liquids on each chunk's 9 by 9 grid.

Cataclysm's split files (`_tex0`, `_obj0`, ...) aren't supported.

## License

This project is licensed under the Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
