use glob::glob;
use id3::Tag;
use karaoke::{log_error, CONFIG};
use lazy_static::lazy_static;
use rayon::prelude::*;
use rustbreak::{deser::Yaml, FileDatabase};
use serde::{Deserialize, Serialize};
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
            Err(e) => {
                log_error(&e);
                std::process::exit(1);
            }
        }
    };
}

pub type CollectionDB = FileDatabase<HashMap<u64, Kfile>, Yaml>;
pub type FavoritesDB = FileDatabase<HashSet<u64>, Yaml>;

pub trait Database {
    type Data;

    fn initialize(path: &PathBuf) -> Result<Box<Self>, failure::Error>;
    fn refresh(&self, path: &PathBuf) -> Result<(), failure::Error>;
    fn data(&self) -> Result<Self::Data, failure::Error>;
}

impl Database for CollectionDB {
    type Data = Collection;

    //If file doesn't exist, create default. Load db from file.
    fn initialize(path: &PathBuf) -> Result<Box<CollectionDB>, failure::Error> {
        let db: CollectionDB;

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
            .map(|path| Kfile::new(path, &CONFIG.song_format))
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

        log::info!(
            "Invalid songs removed: {}",
            missing_valid_keys_to_remove.len()
        );
        log::info!("New songs added: {}", valid_kfiles_to_add.len());

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

    fn data(&self) -> Result<Self::Data, failure::Error> {
        let mut _collection = Vec::new();
        self.read(|db| {
            for value in db.values() {
                _collection.push(value.clone());
            }
        })?;

        let collection = Collection::new(_collection);
        log::info!("# Songs: {}", collection.by_song.len());
        log::info!("# Artists: {}", collection.by_artist.len());

        Ok(collection)
    }
}

impl Database for FavoritesDB {
    type Data = HashSet<u64>;

    //If file doesn't exist, create default. Load db from file.
    fn initialize(path: &PathBuf) -> Result<Box<Self>, failure::Error> {
        let db: FavoritesDB;

        let mut db_path = path.to_path_buf();
        db_path.push("favorites.yaml");

        let exists = db_path.exists();
        db = FavoritesDB::from_path(db_path, HashSet::new())?;
        if !exists {
            db.save()?;
        }
        db.load()?;

        Ok(Box::new(db))
    }

    fn refresh(&self, _path: &PathBuf) -> Result<(), failure::Error> {
        Ok(())
    }

    fn data(&self) -> Result<Self::Data, failure::Error> {
        Ok(self.get_data(true)?)
    }
}

pub fn add_favorite(
    db: impl AsRef<FavoritesDB>,
    hash: u64,
) -> Result<(), rustbreak::RustbreakError> {
    db.as_ref().load()?;

    db.as_ref().write(|favorites| favorites.insert(hash))?;

    db.as_ref().save()?;

    Ok(())
}

pub fn remove_favorite(
    db: impl AsRef<FavoritesDB>,
    hash: u64,
) -> Result<(), rustbreak::RustbreakError> {
    db.as_ref().load()?;

    db.as_ref().write(|favorites| favorites.remove(&hash))?;

    db.as_ref().save()?;

    Ok(())
}

pub fn startup(no_collection_update: bool) -> Result<Collection, failure::Error> {
    let collection_db = CollectionDB::initialize(&CONFIG.data_path)?;
    if !no_collection_update {
        collection_db.refresh(&CONFIG.song_path)?;
    }
    collection_db.data()
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
    fn new(path: &PathBuf, format: &str) -> Kfile {
        let mp3_path = PathBuf::from(path.to_str().unwrap().to_string() + ".mp3");
        let cdg_path = PathBuf::from(path.to_str().unwrap().to_string() + ".cdg");
        let file_name = path.file_name().unwrap().to_str().unwrap();

        let tag = Tag::read_from_path(&mp3_path).unwrap_or_default();
        let tag_artist = tag.artist();
        let tag_song = tag.title();

        let parse_result = std::panic::catch_unwind(|| song_parse(file_name, format));
        let (parsed_artist, parsed_song) = match parse_result {
            Ok(Some(parse)) => (parse.artist, parse.title),
            _ => ("<None>".to_owned(), file_name.to_owned()),
        };

        let artist = match tag_artist {
            Some(s) => s,
            None => &parsed_artist,
        };
        let song = match tag_song {
            Some(s) => s,
            None => &parsed_song,
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

pub struct ParseResult {
    pub title: String,
    pub artist: String,
}

#[derive(Debug)]
enum ParseOrder {
    RemainingArtistTitle,
    RemainingTitleArtist,
    ArtistTitleRemaining,
    ArtistRemainingTitle,
    TitleArtistRemaining,
    TitleRemainingArtist,
    ArtistTitle,
    TitleArtist,
}

#[allow(clippy::cognitive_complexity)]
fn song_parse(file_name: &str, song_format: &str) -> Option<ParseResult> {
    let mut file_name = file_name.to_owned();

    let artist_idx = song_format.find("[Artist]")?;
    let title_idx = song_format.find("[Title]")?;
    let remaining_idx = song_format.find("[*]");

    let parse_order = {
        if let Some(remaining_idx) = remaining_idx {
            if remaining_idx < artist_idx && remaining_idx < title_idx {
                if artist_idx < title_idx {
                    ParseOrder::RemainingArtistTitle
                } else {
                    ParseOrder::RemainingTitleArtist
                }
            } else if artist_idx < remaining_idx && artist_idx < title_idx {
                if remaining_idx < title_idx {
                    ParseOrder::ArtistRemainingTitle
                } else {
                    ParseOrder::ArtistTitleRemaining
                }
            } else if remaining_idx < artist_idx {
                ParseOrder::TitleRemainingArtist
            } else {
                ParseOrder::TitleArtistRemaining
            }
        } else if title_idx < artist_idx {
            ParseOrder::TitleArtist
        } else {
            ParseOrder::ArtistTitle
        }
    };

    let delimiter_start = match parse_order {
        ParseOrder::RemainingArtistTitle => {
            if remaining_idx? > 0 {
                Some(&song_format[..remaining_idx?])
            } else {
                None
            }
        }
        ParseOrder::RemainingTitleArtist => {
            if remaining_idx? > 0 {
                Some(&song_format[..remaining_idx?])
            } else {
                None
            }
        }
        ParseOrder::TitleArtistRemaining => {
            if title_idx > 0 {
                Some(&song_format[..title_idx])
            } else {
                None
            }
        }
        ParseOrder::TitleRemainingArtist => {
            if title_idx > 0 {
                Some(&song_format[..title_idx])
            } else {
                None
            }
        }
        ParseOrder::ArtistRemainingTitle => {
            if artist_idx > 0 {
                Some(&song_format[..artist_idx])
            } else {
                None
            }
        }
        ParseOrder::ArtistTitleRemaining => {
            if artist_idx > 0 {
                Some(&song_format[..artist_idx])
            } else {
                None
            }
        }
        ParseOrder::ArtistTitle => {
            if artist_idx > 0 {
                Some(&song_format[..artist_idx])
            } else {
                None
            }
        }
        ParseOrder::TitleArtist => {
            if title_idx > 0 {
                Some(&song_format[..title_idx])
            } else {
                None
            }
        }
    };

    let delimiter_end = match parse_order {
        ParseOrder::RemainingArtistTitle => {
            if title_idx < song_format.len() - 7 {
                Some(&song_format[(title_idx + 7)..song_format.len()])
            } else {
                None
            }
        }
        ParseOrder::RemainingTitleArtist => {
            if artist_idx < song_format.len() - 8 {
                Some(&song_format[(artist_idx + 8)..song_format.len()])
            } else {
                None
            }
        }
        ParseOrder::TitleArtistRemaining => {
            if remaining_idx? < song_format.len() - 3 {
                Some(&song_format[(remaining_idx? + 3)..song_format.len()])
            } else {
                None
            }
        }
        ParseOrder::TitleRemainingArtist => {
            if artist_idx < song_format.len() - 8 {
                Some(&song_format[(artist_idx + 8)..song_format.len()])
            } else {
                None
            }
        }
        ParseOrder::ArtistRemainingTitle => {
            if title_idx < song_format.len() - 7 {
                Some(&song_format[(title_idx + 7)..song_format.len()])
            } else {
                None
            }
        }
        ParseOrder::ArtistTitleRemaining => {
            if remaining_idx? < song_format.len() - 3 {
                Some(&song_format[(remaining_idx? + 3)..song_format.len()])
            } else {
                None
            }
        }
        ParseOrder::ArtistTitle => {
            if title_idx < song_format.len() - 7 {
                Some(&song_format[(title_idx + 7)..song_format.len()])
            } else {
                None
            }
        }
        ParseOrder::TitleArtist => {
            if artist_idx < song_format.len() - 8 {
                Some(&song_format[(artist_idx + 8)..song_format.len()])
            } else {
                None
            }
        }
    };

    let delimiter_1 = match parse_order {
        ParseOrder::RemainingArtistTitle => &song_format[(remaining_idx? + 3)..artist_idx],
        ParseOrder::RemainingTitleArtist => &song_format[(remaining_idx? + 3)..title_idx],
        ParseOrder::TitleArtistRemaining => &song_format[(title_idx + 7)..artist_idx],
        ParseOrder::TitleRemainingArtist => &song_format[(title_idx + 7)..remaining_idx?],
        ParseOrder::ArtistRemainingTitle => &song_format[(artist_idx + 8)..remaining_idx?],
        ParseOrder::ArtistTitleRemaining => &song_format[(artist_idx + 8)..title_idx],
        ParseOrder::ArtistTitle => &song_format[(artist_idx + 8)..title_idx],
        ParseOrder::TitleArtist => &song_format[(title_idx + 7)..artist_idx],
    };

    let delimiter_2 = match parse_order {
        ParseOrder::RemainingArtistTitle => Some(&song_format[(artist_idx + 8)..title_idx]),
        ParseOrder::RemainingTitleArtist => Some(&song_format[(title_idx + 7)..artist_idx]),
        ParseOrder::TitleArtistRemaining => Some(&song_format[(artist_idx + 8)..remaining_idx?]),
        ParseOrder::TitleRemainingArtist => Some(&song_format[(remaining_idx? + 3)..artist_idx]),
        ParseOrder::ArtistRemainingTitle => Some(&song_format[(remaining_idx? + 3)..title_idx]),
        ParseOrder::ArtistTitleRemaining => Some(&song_format[(title_idx + 7)..remaining_idx?]),
        ParseOrder::ArtistTitle => None,
        ParseOrder::TitleArtist => None,
    };

    if let Some(start) = delimiter_start {
        file_name = file_name.replacen(start, "", 1);
    }

    if let Some(end) = delimiter_end {
        file_name = file_name[..(file_name.len() - end.len())].to_string();
    }

    let delimiter_1_idx = file_name.find(delimiter_1)?;
    let delimiter_2_idx = if let Some(delimiter) = delimiter_2 {
        Some(
            file_name[(delimiter_1_idx + delimiter_1.len())..].find(delimiter)?
                + delimiter_1_idx
                + delimiter_1.len(),
        )
    } else {
        None
    };

    let (title, artist) = match parse_order {
        ParseOrder::RemainingArtistTitle => {
            let artist = &file_name[(delimiter_1_idx + delimiter_1.len())..delimiter_2_idx?];
            let title = &file_name[(delimiter_2_idx? + delimiter_2?.len())..];
            (title, artist)
        }
        ParseOrder::RemainingTitleArtist => {
            let title = &file_name[(delimiter_1_idx + delimiter_1.len())..delimiter_2_idx?];
            let artist = &file_name[(delimiter_2_idx? + delimiter_2?.len())..];
            (title, artist)
        }
        ParseOrder::TitleArtistRemaining => {
            let title = &file_name[..delimiter_1_idx];
            let artist = &file_name[(delimiter_1_idx + delimiter_1.len())..delimiter_2_idx?];
            (title, artist)
        }
        ParseOrder::TitleRemainingArtist => {
            let title = &file_name[..delimiter_1_idx];
            let artist = &file_name[(delimiter_2_idx? + delimiter_2?.len())..];
            (title, artist)
        }
        ParseOrder::ArtistRemainingTitle => {
            let artist = &file_name[..delimiter_1_idx];
            let title = &file_name[(delimiter_2_idx? + delimiter_2?.len())..];
            (title, artist)
        }
        ParseOrder::ArtistTitleRemaining => {
            let artist = &file_name[..delimiter_1_idx];
            let title = &file_name[(delimiter_1_idx + delimiter_1.len())..delimiter_2_idx?];
            (title, artist)
        }
        ParseOrder::ArtistTitle => {
            let artist = &file_name[..delimiter_1_idx];
            let title = &file_name[(delimiter_1_idx + delimiter_1.len())..];
            (title, artist)
        }
        ParseOrder::TitleArtist => {
            let title = &file_name[..delimiter_1_idx];
            let artist = &file_name[(delimiter_1_idx + delimiter_1.len())..];
            (title, artist)
        }
    };

    Some(ParseResult {
        title: title.to_owned(),
        artist: artist.to_owned(),
    })
}

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
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
        let formats = vec![
            "[Artist] - [Title]",
            "[Artist] - [Title] - [*]",
            "[Artist] - [*] - [Title]",
            "[Title] - [Artist]",
            "[Title] - [Artist] - [*]",
            "[Title] - [*] - [Artist]",
            "[*] - [Artist] - [Title]",
            "[*] - [Title] - [Artist]",
            "asdf[*] - [Artist] - [Title]asdf",
            "asdf[*]asdf[Title]asdf[Artist]asdf",
            "asdf[Title]asdf[Artist]asdf[*]asdf",
        ];
        let titles = vec![
            "Artist - Title",
            "Artist - Title - *",
            "Artist - * - Title",
            "Title - Artist",
            "Title - Artist - *",
            "Title - * - Artist",
            "* - Artist - Title",
            "* - Title - Artist",
            "asdf* - Artist - Titleasdf",
            "asdf*asdfTitleasdfArtistasdf",
            "asdfTitleasdfArtistasdf*asdf",
        ];
        for (idx, title) in titles.iter().enumerate() {
            let result =
                std::panic::catch_unwind(|| song_parse(title, formats[idx]).unwrap()).unwrap();
            assert_eq!(result.artist, "Artist");
            assert_eq!(result.title, "Title");
        }
    }

    #[test]
    fn test_unclean_song_parse() {
        let song = "ABCD001 The Testers - Testing 123";
        let format = "[*] - [Artist] - [Title]";
        let result = std::panic::catch_unwind(|| song_parse(song, format));
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_kfile_new() {
        let path = PathBuf::from("ABCD001 - The Testers - Testing 123");
        let config = Config::default();
        let kfile = Kfile::new(&path, &config.song_format);
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
            song_path,
            data_path,
            ..Config::default()
        };
        let initialize = CollectionDB::initialize(&config.data_path);
        assert!(initialize.is_ok());

        let collection = initialize.unwrap();
        let refresh = collection.refresh(&config.song_path);
        assert!(refresh.is_ok());

        remove_file("tests/test_data/db.yaml").unwrap();
    }
}
