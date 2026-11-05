# Changelog

All notable changes to wow-alchemy-mpq are documented here, following
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Changed

- Forked from `wow-mpq` 0.7.0 as `wow-alchemy-mpq`, then rewritten around one reader
  (`Archive`), one chain (`PatchChain`) and one writer (`ArchiveBuilder`).
- Headers, hash, block, hi-block, HET and BET tables, `(attributes)`, sector offset tables and
  patch headers are records derived with `wow-alchemy-data`, written down once for reading and
  writing.
- `Archive::list` only returns names from `(listfile)`; an archive without one lists nothing.

### Fixed

- Encryption keys are computed from the plain file name, as Storm does, so encrypted files in
  folders read.
- Encrypted files stored without compression are decrypted sector by sector.
- Trailing bytes after the last whole dword are no longer decrypted.
- HET hashes use the lowercased name and set the top bit for 64-bit hashes; BET name hashes
  are read bit-packed.
- Multiple compressions are undone in Storm's order (sparse after zlib or bzip2 now works).
- A failed sector decompression is an error instead of a sector of zeros.
- BSD0 patches moving backwards in the old file work, and the RLE size matches Storm's.
- In a `PatchChain`, a delete marker in a higher archive hides the file of lower archives.
- `PatchChain` reads a file from the highest-priority archive that has it, whether or not its
  listfile names it.

### Removed

- Signature verification and generation, archive comparison, rebuild and in-place
  modification, async and memory-mapped readers, buffer pools, parallel readers, SIMD
  hashing, security limits, debug dumps, test data generators, examples and benchmarks.
