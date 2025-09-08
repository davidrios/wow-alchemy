use std::{fs, path::Path};

use rusqlite::{Connection, params_from_iter, types::ToSqlOutput};

use crate::{
    Error, Result, Value, WdbFile,
    dbd::{DbdFile, GameBuild, download::download_dbd, parse_dbd_file},
};

pub fn base_type_to_sqlite_type(base_type: &str) -> Result<&str> {
    Ok(match base_type {
        "locstring" | "string" => "text",
        "int" => "integer",
        "float" => "real",
        _ => {
            return Err(Error::SqliteTableDefinition(format!(
                "unsupported base type {}",
                base_type
            )));
        }
    })
}

pub fn make_table_definition(dbd: &DbdFile, table_name: &str) -> Result<String> {
    let mut table_cols = Vec::new();
    let mut table_fks = Vec::new();
    for field in &dbd.build.fields {
        let field_name = field.name.to_lowercase();

        let Some(column) = &dbd.columns.get(&field.name) else {
            return Err(Error::SqliteTableDefinition(format!(
                "column not found: {}",
                field.name
            )));
        };

        if field.is_array {
            let sqlite_type = base_type_to_sqlite_type(&column.base_type)?;
            for i in 0..field.array_size.unwrap() {
                let col_def = format!("\"{}_{}\" {}", field_name, i, sqlite_type,);
                table_cols.push(col_def);

                if let Some(fk) = &column.foreign_key {
                    table_fks.push(format!(
                        "foreign key (\"{}_{}\") references {}(\"{}\")",
                        field_name,
                        i,
                        fk.table.to_lowercase(),
                        fk.field.to_lowercase()
                    ));
                }
            }
        } else {
            let col_def = format!(
                "\"{}\" {}{}",
                field_name,
                base_type_to_sqlite_type(&column.base_type)?,
                if field.is_key { " primary key" } else { "" }
            );
            table_cols.push(col_def);

            if let Some(fk) = &column.foreign_key {
                table_fks.push(format!(
                    "foreign key (\"{}\") references {}(\"{}\")",
                    field_name,
                    fk.table.to_lowercase(),
                    fk.field.to_lowercase()
                ));
            }
        }
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
            return Err(Error::SqliteTableDefinition(format!(
                "column not found: {}",
                field.name
            )));
        };

        let field_name = field.name.to_lowercase();

        if field.is_array {
            for i in 0..field.array_size.unwrap() {
                let col_def = format!("\"{}_{}\"", field_name, i);
                table_cols.push(col_def);
                row_params.push("?");
            }
        } else {
            let col_def = format!("\"{}\"", field_name);
            table_cols.push(col_def);
            row_params.push("?");
        }
    }

    Ok(format!(
        "insert into {} ({}) values ({})",
        table_name,
        table_cols.join(","),
        row_params.join(",")
    ))
}

pub fn flatten_values(item: &[Value]) -> Vec<ToSqlOutput<'static>> {
    let mut params = Vec::<ToSqlOutput>::new();
    for ii in item {
        match ii {
            crate::Value::Array(values) => {
                for j in values {
                    match j {
                        crate::Value::Array(_) => unreachable!(),
                        _ => params.push(j.clone().into()),
                    }
                }
            }
            _ => params.push(ii.clone().into()),
        }
    }
    params
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

    let mut conn = Connection::open(output_sqlite)?;

    for dir_entry in root_dir {
        let Ok(dir_entry) = dir_entry else { continue };
        let filename: String = dir_entry.file_name().to_string_lossy().into();

        log::info!("Converting table: {filename:?}");
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

        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(&insert_qr)?;

            #[cfg(feature = "parallel")]
            {
                for chunk in crate::lazy::process_parallel(&dir_entry.path(), &dbd, &wdb) {
                    for (idx, values) in chunk.iter().enumerate() {
                        match values {
                            Ok(values) => {
                                stmt.execute(params_from_iter(flatten_values(values)))?;
                            }
                            Err(err) => {
                                println!("{table_name}: item {idx} parse failed: {err}");
                            }
                        }
                    }
                }
            }

            #[cfg(not(feature = "parallel"))]
            {
                let iter = crate::LazyRecordIterator::new(&mut reader, &dbd, &wdb)?;
                for (idx, values) in iter.enumerate() {
                    match values {
                        Ok(values) => {
                            stmt.execute(params_from_iter(flatten_values(&values)))?;
                        }
                        Err(err) => {
                            println!("{table_name}: item {idx} parse failed: {err}");
                        }
                    }
                }
            }
        }

        tx.commit()?;
    }

    Ok(())
}
