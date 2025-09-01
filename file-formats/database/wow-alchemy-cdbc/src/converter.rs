use std::{collections::HashMap, fs, path::Path};

use rusqlite::{Connection, params_from_iter, types::ToSqlOutput};

use crate::{
    Error, LazyRecordIterator, Result, WdbFile,
    dbd::{DbdFile, GameBuild, download::download_dbd, parse_dbd_file},
};

pub fn make_table_definition(dbd: &DbdFile, table_name: &str) -> Result<String> {
    let mut table_cols = Vec::new();
    let mut table_fks = Vec::new();
    for field in &dbd.build.fields {
        let field_name = field.name.to_lowercase();

        let Some(column) = &dbd.columns.get(&field.name) else {
            dbg!(&dbd);
            return Err(Error::SqliteTableDefinition(format!(
                "column not found: {}",
                field.name
            )));
        };

        let mut sqlite_type = match column.base_type.as_str() {
            "locstring" | "string" => "text",
            "int" => "integer",
            "float" => "real",
            _ => {
                return Err(Error::SqliteTableDefinition(format!(
                    "unsupported base type {}",
                    column.base_type
                )));
            }
        };
        if field.is_array {
            sqlite_type = "text";
        } else if let Some(fk) = &column.foreign_key {
            table_fks.push(format!(
                "foreign key (\"{}\") references {}(\"{}\")",
                field_name,
                fk.table.to_lowercase(),
                fk.field.to_lowercase()
            ));
        }

        let col_def = format!(
            "\"{}\" {}{}",
            field_name,
            sqlite_type,
            if field.is_key { " primary key" } else { "" }
        );
        table_cols.push(col_def);
    }

    Ok(format!(
        "CREATE TABLE {} ({}{}{})",
        table_name,
        table_cols.join(","),
        if table_fks.is_empty() { "" } else { "," },
        table_fks.join(",")
    ))
}

pub fn make_insert_query(dbd: &DbdFile, table_name: &str) -> Result<String> {
    let mut table_cols = Vec::new();
    let mut row_params = Vec::new();
    for field in &dbd.build.fields {
        if !dbd.columns.contains_key(&field.name) {
            dbg!(&dbd);
            return Err(Error::SqliteTableDefinition(format!(
                "column not found: {}",
                field.name
            )));
        };

        let col_def = format!("\"{}\"", field.name.to_lowercase());
        table_cols.push(col_def);
        row_params.push("?");
    }

    Ok(format!(
        "insert into {} ({}) values ({})",
        table_name,
        table_cols.join(","),
        row_params.join(",")
    ))
}

/// Convert all dbc files in a folder to a single SQLite database file
pub fn convert_to_sqlite(
    game_build: GameBuild,
    source_dir: &Path,
    output_sqlite: &Path,
) -> Result<()> {
    let root_dir = match source_dir.read_dir() {
        Ok(iter) => iter,
        Err(err) => return Err(err.into()),
    };

    fs::remove_file(output_sqlite).ok();

    let conn = Connection::open(output_sqlite)?;

    let mut dbd_files = HashMap::new();
    for dir_entry in root_dir {
        let Ok(dir_entry) = dir_entry else { continue };
        let filename: String = dir_entry.file_name().to_string_lossy().into();

        let dbd_file = match download_dbd(&filename) {
            Ok(dbd_file) => dbd_file,
            Err(err) => {
                println!("skipping dbd file {:?} due to error {:?}", filename, err);
                continue;
            }
        };

        let mut table_name: String = dbd_file.file_name().unwrap().to_string_lossy().into();
        if let Some(idx) = table_name.find(".") {
            table_name.truncate(idx);
        };
        table_name = table_name.to_lowercase();

        let dbd = match parse_dbd_file(&game_build, &dbd_file) {
            Ok(dbd) => dbd,
            Err(err) => match err {
                Error::NoFieldsForBuild => {
                    println!("Error: {err}");
                    continue;
                }
                _ => return Err(err),
            },
        };

        conn.execute(&make_table_definition(&dbd, &table_name)?, ())?;

        let mut reader = fs::File::open(dir_entry.path())?;
        let Ok(wdb) = WdbFile::wow_read(&mut reader) else {
            println!("error parsing dbc file: {filename}");
            continue;
        };

        let insert_qr = make_insert_query(&dbd, &table_name)?;

        let iter = LazyRecordIterator::new(&mut reader, &dbd, &wdb)?;
        for (idx, item) in iter.enumerate() {
            match item {
                Ok(item) => {
                    conn.execute(
                        &insert_qr,
                        params_from_iter(
                            item.iter()
                                .map(|i| i.clone().into())
                                .collect::<Vec<ToSqlOutput>>(),
                        ),
                    )?;
                }
                Err(err) => {
                    println!("{table_name}: item {idx} parse failed: {err}");
                }
            }
        }

        dbd_files.insert(filename.to_owned(), dbd_file);
    }

    Ok(())
}
