use std::result::Result;
use std::path::PathBuf;
use std::collections::HashMap;
use std::default::Default;

use glob::glob;

use id3::Tag;

use rustbreak::FileDatabase;
use rustbreak::deser::Yaml;

use serde_derive::{Serialize, Deserialize};

use karaoke::config::DB_FILE;


lazy_static! {
    pub static ref COLLECTION: Vec<Kfile> = {
        startup().unwrap()
    };
}

fn all_cdg() -> Vec<PathBuf> {
    let mut vec = Vec::new();
    for file in glob("songs/**/*.cdg").unwrap().filter_map(Result::ok) {
        vec.push(file);
    }
    vec
}

fn valid_cdg_mp3_paths(paths: Vec<PathBuf>) -> (Vec<PathBuf>,Vec<PathBuf>) {
    let mut exists = Vec::new();
    let mut not_exists = Vec::new();
    for mut path in paths {
        let mp3_path = path.with_extension("mp3");
        path.set_extension("");
        if !mp3_path.exists() {
            not_exists.push(path);
        } else {
            exists.push(path);
        }
    }
    ( exists, not_exists )
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Kfile {
    pub key: String,
    pub mp3_path: PathBuf,
    pub cdg_path: PathBuf,
    pub artist: String,
    pub song: String
}

impl Kfile {
    fn new(path: &PathBuf) -> Kfile {
        let key = path.display().to_string();
        let mp3_path = path.with_extension("mp3");
        let cdg_path = path.with_extension("cdg");

        let tag = Tag::read_from_path(&mp3_path).unwrap();
        let artist = tag.artist().unwrap().to_string();
        let song = tag.title().unwrap().to_string();

        Kfile { key, mp3_path, cdg_path, artist, song }
    }   
}

impl Default for Kfile {
    fn default() -> Kfile {
        Kfile {
            key: String::from(""),
            mp3_path: PathBuf::new(), 
            cdg_path: PathBuf::new(),
            artist: String::from(""),
            song: String::from(""),
        }
    }
}


pub fn startup() -> Result<Vec<Kfile>, failure::Error> {
    let cdg_files = all_cdg();
    let (exists, not_exists) = valid_cdg_mp3_paths(cdg_files); 

    let db_file = DB_FILE.to_path_buf();
    let mut db: FileDatabase<HashMap<String, Kfile>, Yaml>;

    if db_file.exists() {
        db = FileDatabase::<HashMap<String, Kfile>, Yaml>::from_path(db_file, HashMap::new())?;
        db.load()?;
    } else {
        db = FileDatabase::<HashMap<String, Kfile>, Yaml>::from_path(db_file, HashMap::new())?;
        db.save()?;
        db.load()?;
    }
    
    let mut existing_keys = Vec::new();
    println!("Read from Database");
    db.read(|db| {        
        for key in db.keys() {
            existing_keys.push(key.clone());
            println!("\t{}", key);
        }    
    })?;

    let keys_to_remove: Vec<PathBuf> = not_exists.iter().filter_map(|k| {
        if existing_keys[..].contains(&k.display().to_string()) {
            Some(k.clone())
        } else { None }
    }).collect();

    let keys_to_add: Vec<PathBuf> = exists.iter().filter_map(|k| {
        if !existing_keys[..].contains(&k.display().to_string()) {
            Some(k.clone())
        } else { None }
    }).collect();

    println!("{:?}", keys_to_remove);
    println!("{:?}", keys_to_add);

    let mut kfiles = Vec::new();
    for path in keys_to_add.iter() {
        let kfile = Kfile::new(path);
        kfiles.push(kfile);
    }

    println!("Writing to Database");
    db.write(|db| {
        for kfile in kfiles {
            let key = kfile.key.clone();
            db.insert(key, kfile);   
        }        
        for key in keys_to_remove.iter() {
            db.remove(&key.display().to_string());
        }
    })?;

    println!("Syncing Database");
    db.save()?;

    let mut collection = Vec::new();
    println!("Read from Database");
    db.read(|db| {        
        for value in db.values() {
            collection.push(value.clone());
        }    
    })?;

    Ok(collection)
}
