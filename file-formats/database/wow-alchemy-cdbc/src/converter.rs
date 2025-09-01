use std::{collections::HashMap, fs, path::Path};

use rusqlite::Connection;

use crate::{
    Error, Result,
    dbd::{DbdFile, GameBuild, download::download_dbd, parse_dbd_file},
};

pub fn make_table_definition(dbd: &DbdFile, table_name: &str) -> Result<String> {
    let mut table_cols = Vec::new();
    let mut table_fks = Vec::new();
    for field in &dbd.build.fields {
        let Some(column) = &dbd.columns.get(&field.name) else {
            dbg!(&dbd);
            return Err(Error::GenericError(format!(
                "column not found: {}",
                field.name
            )));
        };

        let mut sqlite_type = match column.base_type.as_str() {
            "locstring" | "string" => "text",
            "int" => "integer",
            "float" => "real",
            _ => {
                return Err(Error::GenericError(format!(
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
                field.name, fk.table, fk.field
            ));
        }

        let col_def = format!(
            "\"{}\" {}{}",
            field.name,
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

        let dbd = parse_dbd_file(&game_build, &dbd_file)?;
        conn.execute(&make_table_definition(&dbd, &table_name)?, ())?;

        dbd_files.insert(filename.to_owned(), dbd_file);
    }

    // dbg!(dbd_files);
    dbg!(game_build, source_dir, output_sqlite);
    Ok(())
}
