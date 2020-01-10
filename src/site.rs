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

#[derive(Serialize)]
struct JsonStatus {
    status: &'static str,
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

#[derive(Serialize)]
enum DataType {
    #[serde(rename = "songs")]
    Song(Vec<ResponseSong>),
    #[serde(rename = "artists")]
    Artist(Vec<ResponseArtist>),
    #[serde(rename = "queue")]
    Queue(Vec<ResponseSong>),
}

#[derive(Deserialize)]
struct SongParams {
    page: Option<u32>,
    query: Option<String>,
    artist_id: Option<u64>,
}

#[derive(Deserialize)]
struct ArtistParams {
    page: Option<u32>,
    query: Option<String>,
    artist_id: Option<u64>,
}

fn api_songs(
    collection: web::Data<Collection>,
    params: web::Query<SongParams>,
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

    songs.sort_by_key(|song| song.name.to_lowercase());

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
    params: web::Query<ArtistParams>,
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

    artists.sort_by_key(|artist| artist.name.to_lowercase());

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
    let cmd = WorkerCommand::AddQueue { kfile };
    worker_sender.send(cmd).unwrap();
    HttpResponse::Ok().json(JsonStatus { status: "ok" })
}

fn api_playnow(
    form: web::Form<Song>,
    collection: web::Data<Collection>,
    worker_sender: web::Data<Sender<WorkerCommand>>,
) -> HttpResponse {
    let hash = form.hash;
    let kfile = collection.by_song[&hash].clone();
    let cmd = WorkerCommand::PlayNow { kfile };
    worker_sender.send(cmd).unwrap();
    HttpResponse::Ok().json(JsonStatus { status: "ok" })
}

fn api_next(worker_sender: web::Data<Sender<WorkerCommand>>) -> HttpResponse {
    let cmd = WorkerCommand::Next;
    worker_sender.send(cmd).unwrap();
    HttpResponse::Ok().json(JsonStatus { status: "ok" })
}

fn api_clear(worker_sender: web::Data<Sender<WorkerCommand>>) -> HttpResponse {
    let cmd = WorkerCommand::ClearQueue;
    worker_sender.send(cmd).unwrap();
    HttpResponse::Ok().json(JsonStatus { status: "ok" })
}

fn api_stop(worker_sender: web::Data<Sender<WorkerCommand>>) -> HttpResponse {
    let cmd = WorkerCommand::Stop;
    worker_sender.send(cmd).unwrap();
    HttpResponse::Ok().json(JsonStatus { status: "ok" })
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
    std::env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    let port = get_server_port();
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    let server = HttpServer::new(|| {
        let collection = COLLECTION.clone();
        let worker_sender = WORKER_CHANNEL.0.clone();
        let play_queue = PLAY_QUEUE.clone();

        let mut static_path = CONFIG.data_path.clone();
        static_path.push("static");

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
            .service(actix_files::Files::new("/", static_path).index_file("index.html"))
            .default_service(
                // Redirect all to index.html
                web::get().to(serve_index),
            )
    })
    .bind(addr)?;

    println!("Actix has launched from http://0.0.0.0:{}", port);

    server.run()
}
