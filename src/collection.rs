use std::result::Result;
use std::path::PathBuf;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::default::Default;

use glob::glob;

use id3::Tag;

use rustbreak::FileDatabase;
use rustbreak::deser::Yaml;

use serde_derive::{Serialize, Deserialize};

use karaoke::config::{DB_FILE,SONG_DIR};


lazy_static! {
    pub static ref COLLECTION: Collection = {
        startup().unwrap()
    };
}

fn all_cdg() -> Vec<PathBuf> {
    let mut vec = Vec::new();
    let mut glob_path = SONG_DIR.to_path_buf();
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

        //Insert each song into by_song map; put all artist names into Vec.
        let mut artists: Vec<String> = vec_kfile.iter().map(|k| {
            by_song.insert( calculate_hash(&k) , k.clone() );
            k.artist.clone()
        }).collect();
        
        //Get unique artist names
        artists.dedup();
        
        //Create Artist for each artist name, with empty song map
        for artist in artists {
            by_artist.insert( calculate_hash(&artist) , Artist::new(artist) );
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
        Artist { songs, num_songs: 0, name }
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Kfile {
    pub mp3_path: PathBuf,
    pub cdg_path: PathBuf,
    pub artist: String,
    pub artist_hash: u64,
    pub song: String
}

impl Kfile {
    fn new(path: &PathBuf) -> Kfile {
        let mp3_path = PathBuf::from(path.to_str().unwrap().to_string() + ".mp3");
        let cdg_path = PathBuf::from(path.to_str().unwrap().to_string() + ".cdg");
        let file_name = path.file_name().unwrap().to_str().unwrap();

        let tag = Tag::read_from_path(&mp3_path).unwrap_or(id3::Tag::new());
        let artist = tag.artist().unwrap_or("<None>").to_string();
        let artist_hash = calculate_hash(&artist);
        let song = tag.title().unwrap_or(file_name).to_string();

        Kfile { mp3_path, cdg_path, artist, artist_hash, song }
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


fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}


pub fn startup() -> Result<Collection, failure::Error> {
    let cdg_files = all_cdg();
    let valid = valid_cdg_mp3_paths(cdg_files); 

    let db_file = DB_FILE.to_path_buf();
    let mut db: FileDatabase<HashMap<u64, Kfile>, Yaml>;

    if db_file.exists() {
        db = FileDatabase::<HashMap<u64, Kfile>, Yaml>::from_path(db_file, HashMap::new())?;
        db.load()?;
    } else {
        db = FileDatabase::<HashMap<u64, Kfile>, Yaml>::from_path(db_file, HashMap::new())?;
        db.save()?;
        db.load()?;
    }
    
    let mut existing_keys = Vec::new();
    db.read(|db| {        
        for key in db.keys() {
            existing_keys.push(key.clone());
        }    
    })?;

    let mut valid_kfiles = Vec::new();
    for path in valid.iter() {
        let kfile = Kfile::new(path);
        valid_kfiles.push(kfile);
    }
    
    let missing_valid_keys_to_remove: Vec<u64> = existing_keys.iter().filter_map(|k| {
        let valid_keys: Vec<u64> = valid_kfiles.iter().map(|x| { calculate_hash(&x) }).collect();
        if valid_keys.contains(&k) {
            None
        } else { Some(k.clone()) }
    }).collect();

    let valid_kfiles_to_add: Vec<Kfile> = valid_kfiles.iter().filter_map(|k| {
        if !existing_keys[..].contains(&calculate_hash(&k)) {
            Some(k.clone())
        } else { None }
    }).collect();

    println!("Invalid songs removed: {}", missing_valid_keys_to_remove.len());
    println!("New songs added: {}", valid_kfiles_to_add.len());

    db.write(|db| {
        for key in missing_valid_keys_to_remove.iter() {
            db.remove(key);
        }
        for kfile in valid_kfiles_to_add {
            let key = calculate_hash(&kfile);
            db.insert(key, kfile);   
        }        
    })?;

    db.save()?;

    let mut _collection = Vec::new();
    db.read(|db| {        
        for value in db.values() {
            _collection.push(value.clone());
        }    
    })?;

    let collection = Collection::new(_collection);

    println!("# Songs: {}", collection.by_song.len());
    println!("# Artists: {}", collection.by_artist.len());

    Ok(collection)
}
