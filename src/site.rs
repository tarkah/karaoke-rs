use actix_web::{middleware, web, App, Error, HttpResponse, HttpServer};
use crossbeam_channel::Sender;
use karaoke::{
    channel::{WorkerCommand, WORKER_CHANNEL},
    collection::{calculate_hash, Collection, Kfile, COLLECTION},
    queue::PLAY_QUEUE,
    CONFIG,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

const PAGE_SIZE: usize = 100;

#[derive(Deserialize)]
struct Song {
    hash: u64,
}

#[derive(Serialize, Deserialize)]
struct Queue {
    queue: Vec<Kfile>,
}

#[derive(Serialize, Clone)]
struct ResponseSong {
    id: u64,
    name: String,
    artist_id: u64,
    artist_name: String,
}

#[derive(Serialize, Clone)]
struct ResponseArtist {
    id: u64,
    name: String,
    num_songs: usize,
}

#[derive(Serialize)]
struct Response {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<DataType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_pages: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
}

impl Default for Response {
    fn default() -> Response {
        Response {
            status: "ok",
            data: None,
            page: None,
            total_pages: None,
            error_message: None,
        }
    }
}

#[derive(Serialize)]
enum DataType {
    #[serde(rename = "songs")]
    Song(Vec<ResponseSong>),
    #[serde(rename = "artists")]
    Artist(Vec<ResponseArtist>),
    #[serde(rename = "queue")]
    Queue(Vec<ResponseSong>),
    #[serde(rename = "next_song")]
    NextSong { mp3: String, cdg: String },
    #[serde(rename = "player_active")]
    PlayerActive(bool),
}

#[derive(Deserialize)]
struct Params {
    page: Option<u32>,
    query: Option<String>,
    artist_id: Option<u64>,
    sort_key: Option<SortKey>,
    sort_direction: Option<SortDirection>,
}

#[derive(Deserialize, Clone, Copy)]
enum SortKey {
    #[serde(rename = "song")]
    Song,
    #[serde(rename = "artist")]
    Artist,
    #[serde(rename = "numsongs")]
    NumSongs,
}

#[derive(Deserialize, Clone, Copy, PartialEq)]
enum SortDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

fn api_songs(
    collection: web::Data<Collection>,
    params: web::Query<Params>,
) -> Result<web::Json<Response>, Error> {
    let songs = collection.get_ref().by_song.clone();
    let mut songs: Vec<ResponseSong> = songs
        .into_iter()
        .map(|(id, song)| ResponseSong {
            id,
            name: song.song,
            artist_id: song.artist_hash,
            artist_name: song.artist,
        })
        .collect();

    let sort_key = params.sort_key.unwrap_or(SortKey::Song);
    let sort_direction = params.sort_direction.unwrap_or(SortDirection::Asc);
    songs.sort_by_key(|song| match sort_key {
        SortKey::Artist => song.artist_name.to_lowercase(),
        _ => song.name.to_lowercase(),
    });
    if sort_direction == SortDirection::Desc {
        songs.reverse();
    }

    if let Some(artist_id) = params.artist_id {
        songs = songs
            .into_iter()
            .filter(|song| song.artist_id == artist_id)
            .collect();
    }

    if let Some(query) = &params.query {
        songs = songs
            .into_iter()
            .filter(|song| {
                song.name.to_lowercase().contains(&query.to_lowercase())
                    || song
                        .artist_name
                        .to_lowercase()
                        .contains(&query.to_lowercase())
            })
            .collect();
    }

    let page = params.page.unwrap_or(1);
    let song_count = songs.len();
    let pages = (song_count as f32 / PAGE_SIZE as f32).ceil() as u32;

    if page == 0 || (page > pages && pages > 0) {
        let response = Response {
            status: "error",
            error_message: Some("Incorrect page number".to_string()),
            data: None,
            page: None,
            total_pages: None,
        };
        return Ok(web::Json(response));
    }

    songs = songs
        .chunks(PAGE_SIZE)
        .nth((page - 1) as usize)
        .unwrap_or(&[])
        .to_vec();

    let response = Response {
        status: "ok",
        error_message: None,
        data: Some(DataType::Song(songs)),
        page: Some(page),
        total_pages: Some(pages),
    };

    Ok(web::Json(response))
}

fn api_artists(
    collection: web::Data<Collection>,
    params: web::Query<Params>,
) -> Result<web::Json<Response>, Error> {
    let artists = collection.get_ref().by_artist.clone();
    let mut artists: Vec<ResponseArtist> = artists
        .into_iter()
        .map(|(id, artist)| ResponseArtist {
            id,
            name: artist.name,
            num_songs: artist.num_songs,
        })
        .collect();

    if let Some(artist_id) = params.artist_id {
        artists = artists
            .into_iter()
            .filter(|artist| artist.id == artist_id)
            .collect();
    }

    let sort_key = params.sort_key.unwrap_or(SortKey::Artist);
    let sort_direction = params.sort_direction.unwrap_or(SortDirection::Asc);
    artists.sort_by_key(|artist| match sort_key {
        SortKey::NumSongs => format!("{:0>5}", artist.num_songs),
        _ => artist.name.to_lowercase(),
    });
    if sort_direction == SortDirection::Desc {
        artists.reverse();
    }

    if let Some(query) = &params.query {
        artists = artists
            .into_iter()
            .filter(|artist| artist.name.to_lowercase().contains(&query.to_lowercase()))
            .collect();
    }

    let page = params.page.unwrap_or(1);
    let artist_count = artists.len();
    let pages = (artist_count as f32 / PAGE_SIZE as f32).ceil() as u32;

    if page == 0 || (page > pages && pages > 0) {
        let response = Response {
            status: "error",
            error_message: Some("Incorrect page number".to_string()),
            data: None,
            page: None,
            total_pages: None,
        };
        return Ok(web::Json(response));
    }

    artists = artists
        .chunks(PAGE_SIZE)
        .nth((page - 1) as usize)
        .unwrap_or(&[])
        .to_vec();

    let response = Response {
        status: "ok",
        error_message: None,
        data: Some(DataType::Artist(artists)),
        page: Some(page),
        total_pages: Some(pages),
    };

    Ok(web::Json(response))
}

fn api_queue(queue: web::Data<Arc<Mutex<Vec<Kfile>>>>) -> Result<web::Json<Response>, Error> {
    let queue = queue.get_ref().as_ref().lock().unwrap().clone();
    let queue: Vec<ResponseSong> = queue
        .into_iter()
        .map(|kfile| {
            let id = calculate_hash(&kfile);

            ResponseSong {
                id,
                name: kfile.song,
                artist_name: kfile.artist,
                artist_id: kfile.artist_hash,
            }
        })
        .collect();

    let response = Response {
        status: "ok",
        error_message: None,
        data: Some(DataType::Queue(queue)),
        page: None,
        total_pages: None,
    };

    Ok(web::Json(response))
}

fn api_add(
    form: web::Form<Song>,
    collection: web::Data<Collection>,
    worker_sender: web::Data<Sender<WorkerCommand>>,
) -> HttpResponse {
    let hash = form.hash;
    let kfile = collection.by_song[&hash].clone();
    log::info!("Song added to queue: {} - {}", kfile.artist, kfile.song);
    let cmd = WorkerCommand::AddQueue { kfile };
    worker_sender.send(cmd).unwrap();
    HttpResponse::Ok().json(Response {
        status: "ok",
        ..Response::default()
    })
}

fn api_playnow(
    form: web::Form<Song>,
    collection: web::Data<Collection>,
    worker_sender: web::Data<Sender<WorkerCommand>>,
) -> HttpResponse {
    let hash = form.hash;
    let kfile = collection.by_song[&hash].clone();
    log::info!("Play now requested for: {} - {}", kfile.artist, kfile.song);
    let cmd = WorkerCommand::PlayNow { kfile };
    worker_sender.send(cmd).unwrap();
    HttpResponse::Ok().json(Response {
        status: "ok",
        ..Response::default()
    })
}

fn api_next(worker_sender: web::Data<Sender<WorkerCommand>>) -> HttpResponse {
    let cmd = WorkerCommand::Next;
    log::info!("Next song requested");
    worker_sender.send(cmd).unwrap();
    HttpResponse::Ok().json(Response {
        status: "ok",
        ..Response::default()
    })
}

fn api_clear(worker_sender: web::Data<Sender<WorkerCommand>>) -> HttpResponse {
    let cmd = WorkerCommand::ClearQueue;
    log::info!("Queue clear requested");
    worker_sender.send(cmd).unwrap();
    HttpResponse::Ok().json(Response {
        status: "ok",
        ..Response::default()
    })
}

fn api_stop(worker_sender: web::Data<Sender<WorkerCommand>>) -> HttpResponse {
    let cmd = WorkerCommand::Stop;
    log::info!("Stop requested");
    worker_sender.send(cmd).unwrap();
    HttpResponse::Ok().json(Response {
        status: "ok",
        ..Response::default()
    })
}

fn api_player() -> HttpResponse {
    let active = CONFIG.use_web_player;

    HttpResponse::Ok().json(Response {
        status: "ok",
        data: Some(DataType::PlayerActive(active)),
        ..Response::default()
    })
}

fn api_player_next(queue: web::Data<Arc<Mutex<Vec<Kfile>>>>) -> HttpResponse {
    let _queue = queue.lock().unwrap();
    if _queue.len() == 0 {
        drop(_queue);
        return HttpResponse::Ok().json(Response {
            status: "error",
            error_message: Some("no songs in queue".to_string()),
            ..Response::default()
        });
    }

    let mp3 = _queue[0]
        .mp3_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let cdg = _queue[0]
        .cdg_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    drop(_queue);

    HttpResponse::Ok().json(Response {
        status: "ok",
        data: Some(DataType::NextSong { mp3, cdg }),
        ..Response::default()
    })
}

fn api_player_ended(queue: web::Data<Arc<Mutex<Vec<Kfile>>>>) -> HttpResponse {
    log::info!("Web player has finished song");

    let mut _queue = queue.lock().unwrap();
    if _queue.len() == 0 {
        drop(_queue);
        return HttpResponse::Ok().json(Response {
            status: "ok",
            ..Response::default()
        });
    }

    _queue.remove(0);
    drop(_queue);

    HttpResponse::Ok().json(Response {
        status: "ok",
        ..Response::default()
    })
}

fn serve_index() -> Result<actix_files::NamedFile, Error> {
    let mut path = CONFIG.data_path.clone();
    path.push("static/index.html");
    Ok(actix_files::NamedFile::open(path)?)
}

fn get_server_port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080)
}

pub fn run() -> std::io::Result<()> {
    let port = get_server_port();
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    let server = HttpServer::new(|| {
        let collection = COLLECTION.clone();
        let worker_sender = WORKER_CHANNEL.0.clone();
        let play_queue = PLAY_QUEUE.clone();

        let mut static_path = CONFIG.data_path.clone();
        static_path.push("static");

        let song_path = CONFIG.song_path.clone();

        App::new()
            .data(collection)
            .data(worker_sender)
            .data(play_queue)
            .wrap(middleware::Logger::default()) // enable logger
            .service(web::resource("/api/add").route(web::post().to(api_add)))
            .service(web::resource("/api/playnow").route(web::post().to(api_playnow)))
            .service(web::resource("/api/next").route(web::post().to(api_next)))
            .service(web::resource("/api/clear").route(web::post().to(api_clear)))
            .service(web::resource("/api/stop").route(web::post().to(api_stop)))
            .service(web::resource("/api/songs").route(web::get().to(api_songs)))
            .service(web::resource("/api/artists").route(web::get().to(api_artists)))
            .service(web::resource("/api/queue").route(web::get().to(api_queue)))
            .service(web::resource("/api/player").route(web::get().to(api_player)))
            .service(web::resource("/api/player/next").route(web::get().to(api_player_next)))
            .service(web::resource("/api/player/ended").route(web::post().to(api_player_ended)))
            .service(actix_files::Files::new("/songs/", song_path))
            .service(actix_files::Files::new("/", static_path).index_file("index.html"))
            .default_service(
                // Redirect all to index.html
                web::get().to(serve_index),
            )
    })
    .bind(addr)?;

    log::info!("Website has launched from http://0.0.0.0:{}", port);

    server.run()
}
