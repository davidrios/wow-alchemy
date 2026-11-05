# wow-alchemy-cdbc

Reader and writer for World of Warcraft client database tables in the WDBC format (every
`.dbc` up to Cataclysm, 3.3.5a's included) and WDB2 (Cataclysm and Mists of Pandaria `.db2`).
Legion's WDB3, WDB4, WDB5 and later are refused.

- `WdbFile::from_bytes` reads a table; `field_u32` and `string_bytes` read raw fields and
  strings by their offset, and `to_bytes` writes it back unchanged.
- With a [WoWDBDefs](https://github.com/wowdev/WoWDBDefs) definition (`dbd`, downloaded and
  cached by `dbd::download::download_dbd`), `read_records` gives typed records.
- With the `sqlite` feature, `sqlite_converter::convert_to_sqlite` turns a folder of tables into
  one SQLite database.

## License

This project is licensed under the Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
