use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ApiResponse {
    SuccessGet(SuccessGetResponse),
    SuccessPost(SuccessPostResponse),
    Error(ErrorResponse),
}

#[derive(Deserialize, Debug)]
pub struct SuccessGetResponse {
    pub status: String,
    pub data: DataType,
    pub page: Option<u32>,
    pub total_pages: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct SuccessPostResponse {
    pub status: String,
}
#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub status: String,
    pub error_message: String,
}

#[derive(Deserialize, Debug)]
pub enum DataType {
    #[serde(rename = "songs")]
    Songs(Vec<Song>),
    #[serde(rename = "artists")]
    Artists(Vec<Artist>),
    #[serde(rename = "queue")]
    Queue(Vec<Song>),
    #[serde(rename = "next_song")]
    PlayerNextSong { mp3: String, cdg: String },
    #[serde(rename = "config")]
    Config(Config),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Song {
    pub id: u64,
    pub name: String,
    pub artist_id: u64,
    pub artist_name: String,
    pub favorite: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Artist {
    pub id: u64,
    pub name: String,
    pub num_songs: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_key: Option<SortKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_direction: Option<SortDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorites_only: Option<bool>,
}

impl Default for RequestParams {
    fn default() -> RequestParams {
        RequestParams {
            page: None,
            query: None,
            artist_id: None,
            sort_key: None,
            sort_direction: None,
            favorites_only: None,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct PostSong {
    pub hash: u64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum SortKey {
    #[serde(rename = "song")]
    Song,
    #[serde(rename = "artist")]
    Artist,
    #[serde(rename = "numsongs")]
    NumSongs,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    pub song_path: PathBuf,
    pub data_path: PathBuf,
    pub no_collection_update: bool,
    pub use_web_player: bool,
    pub port: u16,
    pub port_ws: u16,
    pub song_format: String,
    pub player: PlayerConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PlayerConfig {
    pub fullscreen: bool,
    pub scale: f32,
    pub disable_background: bool,
}
