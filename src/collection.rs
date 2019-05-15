use glob::glob;
use id3::Tag;
use karaoke::CONFIG;
use lazy_static::lazy_static;
use rayon::prelude::*;
use rustbreak::{deser::Yaml, FileDatabase};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    default::Default,
    hash::{Hash, Hasher},
    path::PathBuf,
    result::Result,
};

lazy_static! {
    pub static ref COLLECTION: Collection = {
        let collection = startup(CONFIG.no_collection_update);
        match collection {
            Ok(c) => c,
            Err(e) => panic!("{}", e),
        }
    };
}

pub type CollectionDB = FileDatabase<HashMap<u64, Kfile>, Yaml>;

pub trait Custom {
    fn initialize(path: &PathBuf) -> Result<Box<Self>, failure::Error>;
    fn refresh(&self, path: &PathBuf) -> Result<(), failure::Error>;
    fn get_collection(&self) -> Result<Collection, failure::Error>;
}

impl Custom for CollectionDB {
    //If file doesn't exist, create default. Load db from file.
    fn initialize(path: &PathBuf) -> Result<Box<CollectionDB>, failure::Error> {
        let mut db: CollectionDB;

        let mut db_path = path.to_path_buf();
        db_path.push("db.yaml");

        let exists = db_path.exists();
        db = CollectionDB::from_path(db_path, HashMap::new())?;
        if !exists {
            db.save()?;
        }
        db.load()?;

        Ok(Box::new(db))
    }

    fn refresh(&self, song_path: &PathBuf) -> Result<(), failure::Error> {
        let cdg_files = all_cdg(&song_path);
        let valid = valid_cdg_mp3_paths(cdg_files);

        let mut existing_keys = Vec::new();
        self.read(|db| {
            for key in db.keys() {
                existing_keys.push(key.clone());
            }
        })?;

        let valid_kfiles = valid
            .par_iter()
            .map(|path| Kfile::new(path))
            .collect::<Vec<Kfile>>();

        let missing_valid_keys_to_remove: Vec<u64> = existing_keys
            .par_iter()
            .filter_map(|k| {
                let valid_keys: Vec<u64> = valid_kfiles
                    .par_iter()
                    .map(|x| calculate_hash(&x))
                    .collect();
                if valid_keys.contains(&k) {
                    None
                } else {
                    Some(*k)
                }
            })
            .collect();

        let valid_kfiles_to_add: Vec<Kfile> = valid_kfiles
            .par_iter()
            .filter_map(|k| {
                if !existing_keys[..].contains(&calculate_hash(&k)) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();

        println!(
            "Invalid songs removed: {}",
            missing_valid_keys_to_remove.len()
        );
        println!("New songs added: {}", valid_kfiles_to_add.len());

        self.write(|db| {
            for key in missing_valid_keys_to_remove {
                db.remove(&key);
            }
            for kfile in valid_kfiles_to_add {
                let key = calculate_hash(&kfile);
                db.insert(key, kfile);
            }
        })?;

        self.save()?;

        Ok(())
    }

    fn get_collection(&self) -> Result<Collection, failure::Error> {
        let mut _collection = Vec::new();
        self.read(|db| {
            for value in db.values() {
                _collection.push(value.clone());
            }
        })?;

        let collection = Collection::new(_collection);
        println!("# Songs: {}", collection.by_song.len());
        println!("# Artists: {}", collection.by_artist.len());

        Ok(collection)
    }
}

pub fn startup(no_collection_update: bool) -> Result<Collection, failure::Error> {
    let collection_db = CollectionDB::initialize(&CONFIG.data_path)?;
    if !no_collection_update {
        collection_db.refresh(&CONFIG.song_path)?;
    }
    collection_db.get_collection()
}

fn all_cdg(song_path: &PathBuf) -> Vec<PathBuf> {
    let mut vec = Vec::new();
    let mut glob_path = song_path.to_path_buf();
    glob_path.push("**/*.cdg");
    let glob_str = glob_path.display().to_string();
    for file in glob(&glob_str).unwrap().filter_map(Result::ok) {
        vec.push(file);
    }
    vec
}

fn valid_cdg_mp3_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut valid = Vec::new();
    for mut path in paths {
        let mp3_path = path.with_extension("mp3");
        path.set_extension("");
        if mp3_path.exists() {
            valid.push(path);
        }
    }
    valid
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    pub by_song: HashMap<u64, Kfile>,
    pub by_artist: HashMap<u64, Artist>,
}

impl Collection {
    fn new(vec_kfile: Vec<Kfile>) -> Collection {
        let mut by_song = HashMap::new();
        let mut by_artist = HashMap::new();

        //Insert each song into by_song map; collect unique all artist names
        let artists: HashSet<String> = vec_kfile
            .into_iter()
            .map(|k| {
                by_song.insert(calculate_hash(&k), k.clone());
                k.artist
            })
            .collect();

        //Create Artist for each artist name, with empty song map
        for artist in artists {
            by_artist.insert(calculate_hash(&artist), Artist::new(artist));
        }

        //Insert applicable songs into each artist song map
        for (artist_hash, artist) in by_artist.iter_mut() {
            for (kfile_hash, kfile) in by_song.iter() {
                if kfile.artist_hash == *artist_hash {
                    artist.songs.insert(kfile_hash.clone(), kfile.clone());
                }
            }
            let num_songs = artist.songs.len();
            artist.num_songs = num_songs;
        }

        Collection { by_song, by_artist }
    }
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Artist {
    pub songs: HashMap<u64, Kfile>,
    pub num_songs: usize,
    pub name: String,
}

impl Artist {
    fn new(name: String) -> Artist {
        let songs = HashMap::new();
        Artist {
            songs,
            num_songs: 0,
            name,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Kfile {
    pub mp3_path: PathBuf,
    pub cdg_path: PathBuf,
    pub artist: String,
    pub artist_hash: u64,
    pub song: String,
}

impl Kfile {
    fn new(path: &PathBuf) -> Kfile {
        let mp3_path = PathBuf::from(path.to_str().unwrap().to_string() + ".mp3");
        let cdg_path = PathBuf::from(path.to_str().unwrap().to_string() + ".cdg");
        let file_name = path.file_name().unwrap().to_str().unwrap();

        let tag = Tag::read_from_path(&mp3_path).unwrap_or_default();
        let tag_artist = tag.artist();
        let tag_song = tag.title();

        let parsed_file = song_parse(file_name);
        let (parsed_artist, parsed_song) = match parsed_file {
            Some(tuple) => (tuple.0, tuple.1),
            None => ("<None>", file_name),
        };

        let artist = match tag_artist {
            Some(s) => s,
            None => parsed_artist,
        };
        let song = match tag_song {
            Some(s) => s,
            None => parsed_song,
        };

        let artist_hash = calculate_hash(&artist);

        Kfile {
            mp3_path,
            cdg_path,
            artist: artist.to_string(),
            artist_hash,
            song: song.to_string(),
        }
    }
}

impl Default for Kfile {
    fn default() -> Kfile {
        Kfile {
            mp3_path: PathBuf::new(),
            cdg_path: PathBuf::new(),
            artist: String::from(""),
            artist_hash: calculate_hash(&String::from("")),
            song: String::from(""),
        }
    }
}

//Parses artist & song from files with naming convention: "album - artist - song"
fn song_parse(file_name: &str) -> Option<(&str, &str)> {
    let mut split: Vec<&str> = file_name.split(" - ").collect();

    if split.len() == 3 {
        let song = split.pop().unwrap();
        let artist = split.pop().unwrap();
        Some((artist, song))
    } else {
        None
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use karaoke::config::Config;
    use std::{fs::remove_file, path::PathBuf};

    #[test]
    fn test_all_cdg() {
        let song_path = PathBuf::from("tests/test_data/songs");
        let all_cdg = all_cdg(&song_path);
        let count = all_cdg.len();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_valid_cdg_mp3_paths() {
        let song_path = PathBuf::from("tests/test_data/songs");
        let all_cdg = all_cdg(&song_path);
        let valid = valid_cdg_mp3_paths(all_cdg);
        let count = valid.len();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_clean_song_parse() {
        let parse_string = "ABCD001 - The Testers - Testing 123";
        let parsed_file = song_parse(parse_string);
        assert_eq!(parsed_file, Some(("The Testers", "Testing 123")));
    }

    #[test]
    fn test_unclean_song_parse() {
        let parse_string = "ABCD001 The Testers - Testing 123";
        let parsed_file = song_parse(parse_string);
        assert_eq!(parsed_file, None);
    }

    #[test]
    fn test_kfile_new() {
        let path = PathBuf::from("ABCD001 - The Testers - Testing 123");
        let kfile = Kfile::new(&path);
        let _kfile = Kfile {
            mp3_path: PathBuf::from("ABCD001 - The Testers - Testing 123.mp3"),
            cdg_path: PathBuf::from("ABCD001 - The Testers - Testing 123.cdg"),
            artist: String::from("The Testers"),
            artist_hash: calculate_hash(&String::from("The Testers")),
            song: String::from("Testing 123"),
        };
        assert_eq!(kfile, _kfile);
    }

    #[test]
    fn test_startup() {
        let song_path = PathBuf::from("tests/test_data/songs");
        let data_path = PathBuf::from("tests/test_data");
        let config = Config {
            song_path: song_path.to_path_buf(),
            data_path: data_path.to_path_buf(),
            no_collection_update: false,
        };
        let initialize = CollectionDB::initialize(&config.data_path);
        assert!(initialize.is_ok());

        let collection = initialize.unwrap();
        let refresh = collection.refresh(&config.song_path);
        assert!(refresh.is_ok());

        remove_file("tests/test_data/db.yaml").unwrap();
    }
}