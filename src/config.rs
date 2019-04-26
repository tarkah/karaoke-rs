// This just reads an example configuration.
// If it doesn't find one, it uses your default configuration
//
// You can create one by writing this file to `$XDG_CONFIG_HOME/rkaraoke/config.yaml`:
// ```
// ---
// data_path: $XDG_DATA_HOME/rkaraoke
// allow_overwrite: true
// ```
//
use std::path::PathBuf;
use std::default::Default;
use rustbreak::FileDatabase;
use rustbreak::deser::Yaml;
use std::fs::DirBuilder;
use dirs::{config_dir, data_dir};
use serde_derive::{Deserialize, Serialize};

type DB = FileDatabase<Config, Yaml>;

lazy_static! {
    pub static ref CONF_DIR: PathBuf = { 
        let mut dir = config_dir().unwrap();
        dir.push("rkaraoke");
        DirBuilder::new()
               .recursive(true)
               .create(dir.clone())
               .unwrap();
        dir
    };

    pub static ref CONF_FILE: PathBuf = { 
        let mut path = CONF_DIR.to_path_buf();
        path.push("config.yaml");
        path
    };

    pub static ref DATA_DIR: PathBuf = { 
        let mut dir = data_dir().unwrap();
        dir.push("rkaraoke");
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

    pub static ref DB_FILE: PathBuf = { 
        let mut path = DATA_DIR.to_path_buf();
        path.push("db.yaml");
        path
    };

    pub static ref CONFIG: DB = {
        let mut db: DB;
        if !CONF_FILE.to_path_buf().exists() {
            db = FileDatabase::from_path(CONF_FILE.to_path_buf(), Config::default())
                .expect("Create database from path");
            db.save().expect("Saving default config");
            db.load().expect("Config to load");
        } else {
            db = FileDatabase::from_path(CONF_FILE.to_path_buf(), Config::default())
                .expect("Create database from path");
            db.load().expect("Config to load");
        }
        db
    };
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub data_path: PathBuf,
    pub allow_overwrite: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            data_path: DATA_DIR.to_path_buf(),
            allow_overwrite: false,
        }
    }
}
