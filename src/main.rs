extern crate self as karaoke;

use clap::{App, Arg};
use karaoke::{
    collection::COLLECTION,
    config::{load_config, Config},
};
use lazy_static::lazy_static;
use std::{fs::metadata, path::PathBuf};

mod channel;
mod collection;
mod config;
mod embed;
mod player;
mod queue;
mod site;
mod worker;

lazy_static! {
    pub static ref CONFIG: Config = {
        let config = get_config();
        match config {
            Ok(c) => c,
            Err(e) => panic!("{}", e),
        }
    };
}

fn main() -> Result<(), failure::Error> {
    lazy_static::initialize(&CONFIG);
    lazy_static::initialize(&COLLECTION);
    karaoke::embed::unload_files();
    karaoke::player::run();
    karaoke::worker::run();
    karaoke::site::run()?;
    Ok(())
}

fn get_config() -> Result<Config, failure::Error> {
    let matches = App::new("karoake-rs")
        .version("0.5.0")
        .author("Cory F. <cforsstrom18@gmail.com>")
        .about("A simple, network enabled karaoke player in Rust")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("songs")
                .short("s")
                .long("songs")
                .value_name("DIR")
                .help("Sets a custom song directory")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("data")
                .short("d")
                .long("data")
                .value_name("DIR")
                .help("Sets a custom data directory")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("no-collection-update")
                .long("no-collection-update")
                .help("Disable collection update on startup"),
        )
        .get_matches();

    let config_path: Option<PathBuf>;
    let song_path: Option<PathBuf>;
    let data_path: Option<PathBuf>;

    //Return each path if valid, panic if not
    let _config_path = matches.value_of("config");
    config_path = match _config_path {
        Some(path) => validate_file(path),
        None => None,
    };
    let _song_path = matches.value_of("songs");
    song_path = match _song_path {
        Some(path) => validate_dir(path),
        None => None,
    };
    let _data_path = matches.value_of("data");
    data_path = match _data_path {
        Some(path) => validate_dir(path),
        None => None,
    };
    let no_collection_update = if matches.is_present("no-collection-update") {
        Some(true)
    } else {
        None
    };

    //Load config file from config_path, override config with supplied Args, if applicable
    load_config(config_path, song_path, data_path, no_collection_update)
}

fn validate_file(path: &str) -> Option<PathBuf> {
    let meta = metadata(path).unwrap();
    let permissions = meta.permissions();
    if !meta.is_file() {
        panic!("File supplied as argument is not valid: {}", path)
    }
    if permissions.readonly() {
        panic!("Do you have permissions for: {}", path)
    }
    Some(PathBuf::from(path))
}

fn validate_dir(path: &str) -> Option<PathBuf> {
    let meta = metadata(path).unwrap();
    let permissions = meta.permissions();
    if !meta.is_dir() {
        panic!("Dir supplied as argument is not valid: {}", path)
    }
    if permissions.readonly() {
        panic!("Do you have permissions for: {}", path)
    }
    Some(PathBuf::from(path))
}
