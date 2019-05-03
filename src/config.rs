// Loads configuration file, overriding values with supplied command line args
// If it doesn't find one, it uses a default configuration
//
// Default configuration is created at `$XDG_CONFIG_HOME/karaoke-rs/config.yaml` as:
// ```
// ---
// song_path: $XDG_DATA_HOME/karaoke-rs/songs
// data_path: $XDG_DATA_HOME/karaoke-rs
// ```
//
use dirs::{config_dir, data_dir};
use lazy_static::lazy_static;
use rustbreak::{deser::Yaml, FileDatabase};
use serde_derive::{Deserialize, Serialize};
use std::{default::Default, fs::DirBuilder, path::PathBuf};

type ConfigDB = FileDatabase<Config, Yaml>;

//Default locations, overriden if supplied in Config file or by Argument
lazy_static! {
    pub static ref CONF_FILE: PathBuf = {
        let mut config_dir = config_dir().unwrap();
        config_dir.push("karaoke-rs");
        DirBuilder::new()
            .recursive(true)
            .create(config_dir.clone())
            .unwrap();

        let mut path = config_dir.to_path_buf();
        path.push("config.yaml");
        path
    };
    pub static ref DATA_DIR: PathBuf = {
        let mut dir = data_dir().unwrap();
        dir.push("karaoke-rs");
        DirBuilder::new()
            .recursive(true)
            .create(dir.clone())
            .unwrap();
        dir
    };
    pub static ref SONG_DIR: PathBuf = {
        let mut dir = DATA_DIR.to_path_buf();
        dir.push("songs");
        dir
    };
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Config {
    pub song_path: PathBuf,
    pub data_path: PathBuf,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            song_path: SONG_DIR.to_path_buf(),
            data_path: DATA_DIR.to_path_buf(),
        }
    }
}

//If file doesn't exist, create default. Load db from file.
fn initialize_db(db_path: PathBuf) -> Result<ConfigDB, failure::Error> {
    let mut db: ConfigDB;

    let exists = db_path.to_path_buf().exists();
    db = FileDatabase::from_path(db_path.to_path_buf(), Config::default())?;
    if !exists {
        db.save()?;
    }
    db.load()?;

    Ok(db)
}

//Loads a configuration file from default / supplied path, then overrides contents with any supplied args
pub fn load_config(
    config_path: Option<PathBuf>,
    song_path: Option<PathBuf>,
    data_path: Option<PathBuf>,
) -> Result<Config, failure::Error> {

    //If config_path supplied (from Arg), use that over default location
    let config_file: PathBuf;
    match config_path {
        Some(path) => {
            config_file = path.to_path_buf();
        }
        None => {
            config_file = CONF_FILE.to_path_buf();
        }
    }
    println!("Using config file: {:?}", config_file.display());

    let db = initialize_db(config_file.to_path_buf())?;

    //get Config struct from db
    let mut config = db.get_data(false)?;

    //Update config with supplied Args
    if let Some(path) = song_path {
        config.song_path = path.to_path_buf();
    }
    if let Some(path) = data_path {
        config.data_path = path.to_path_buf();
    }
    println!("Using song dir: {:?}", config.song_path);
    println!("Using data dir: {:?}", config.data_path);

    Ok(config)
}