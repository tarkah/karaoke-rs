use karaoke::CONFIG;
use rust_embed::RustEmbed;
use std::{
    fs::{write, DirBuilder},
    path::PathBuf,
};

#[derive(RustEmbed)]
#[folder = "embed/static"]
struct Static;

#[derive(RustEmbed)]
#[folder = "embed/config"]
struct Config;

#[derive(RustEmbed)]
#[folder = "assets"]
pub struct Assets;

pub fn unload_files() {
    let nested_folders = vec!["static/vendor/js", "static/vendor/css"];

    for folder in nested_folders {
        //Create static folder in data path if not already exists
        let mut nested_path = CONFIG.data_path.clone();
        nested_path.push(folder);
        if !nested_path.is_dir() {
            DirBuilder::new()
                .recursive(true)
                .create(nested_path.clone())
                .unwrap();
        }
    }

    let mut static_path = CONFIG.data_path.clone();
    static_path.push("static");

    //Save each file into static path
    for file in Static::iter() {
        let file_data = Static::get(file.as_ref()).unwrap();
        let mut path = static_path.clone();
        path.push(file.as_ref());
        write(path.as_path(), file_data.as_ref()).unwrap();
    }
}

pub fn create_config_if_not_exists(config_path: &PathBuf) -> Result<(), failure::Error> {
    if !config_path.exists() {
        let config = Config::get("config.yaml").unwrap();
        write(config_path.as_path(), config.as_ref())?;
    }
    Ok(())
}
