# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [0.2.0] - 2025-09-10

- Changed default features of the main CLI to include everything


### wow-alchemy-cdbc

- Entirely refactored DBC files handling
- Implemented downloading and using DBD files for parsing
- Implemented converting to SQLite as the only export option


## [0.1.0] - 2025-08-30

- Fork from `wowemulation-dev/warcraft-rs` v0.3.1
- Initial clean-up of LLM trash from original repo
- Removed MPQ files support
- Implement `wow-alchemy-data` and `wow-alchemy-data-derive` abstractions for declarative parsing of binary formats
- Reimplement M2 model parsing and added basic support for chunked versions
- Reimplement `skin` file parsing
- Implement `phys` file parsing
