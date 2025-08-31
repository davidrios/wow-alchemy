use std::{collections::HashMap, path::Path};

use crate::{
    Error, Result,
    dbd::{GameBuild, download::download_dbd, parse_dbd_file},
};

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

        let dbd = parse_dbd_file(&game_build, &dbd_file)
            .map_err(|err| Error::GenericError(err.to_string()))?;
        dbg!(dbd);

        dbd_files.insert(filename.to_owned(), dbd_file);
        break;
    }

    dbg!(dbd_files);
    dbg!(game_build, source_dir, output_sqlite);
    todo!()
}
