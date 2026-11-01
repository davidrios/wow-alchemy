# wow-alchemy-ffi

A C ABI for reading World of Warcraft MPQ archives, BLP textures and DBC files from any language. The declarations and ownership rules are in [`include/wow_alchemy.h`](include/wow_alchemy.h).

Build the shared library with:

```bash
cargo build --release -p wow-alchemy-ffi
```

## Conventions

- Functions that can fail return a `WaStatus`. `wa_last_error` describes the most recent failure on the calling thread.
- Handles and buffers returned to you are yours to free with the matching `*_free` function.
- Handles can be used from any thread. Reads on one MPQ chain run one at a time.
- Panics never cross the boundary. They're reported as `WA_ERROR`.

## License

This project is licensed under the Apache License, Version 2.0.
