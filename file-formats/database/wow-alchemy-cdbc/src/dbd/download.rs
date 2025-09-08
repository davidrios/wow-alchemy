use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use directories::ProjectDirs;

use crate::Result;

use super::file_map::get_dbd_name;

pub fn download_dbd(file_name: &str) -> Result<PathBuf> {
    let dbd_file_name = get_dbd_name(file_name)?;

    let proj_dirs = match ProjectDirs::from("com.github", "davidrios", "wow-alchemy-cdbc") {
        Some(proj_dirs) => proj_dirs,
        None => return Err(io::Error::new(io::ErrorKind::NotFound, "can't get cache dir").into()),
    };

    let cache_dir = proj_dirs.cache_dir().join("dbd");
    match fs::create_dir_all(&cache_dir) {
        Ok(_) => {}
        Err(err) => return Err(err.into()),
    }

    let dbd_file_path = cache_dir.join(dbd_file_name);
    if !dbd_file_path.exists() {
        let dbd_file_url = format!(
            "https://raw.githubusercontent.com/wowdev/WoWDBDefs/refs/heads/master/definitions/{}",
            dbd_file_name
        );
        let response = reqwest::blocking::get(dbd_file_url)?;
        let content = response.bytes()?;

        let mut downloaded_file = fs::File::create(&dbd_file_path)?;
        downloaded_file.write_all(&content)?;
    }

    Ok(dbd_file_path)
}
