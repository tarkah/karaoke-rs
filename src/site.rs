use actix_web::{error, guard, middleware, web, App, Error, HttpResponse, HttpServer};
use crossbeam_channel::Sender;
use karaoke::{
    channel::{WorkerCommand, WORKER_CHANNEL},
    collection::{Collection, Kfile, COLLECTION},
    queue::PLAY_QUEUE,
    CONFIG,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};
use tera::Context;

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
}

fn index(tera: web::Data<tera::Tera>) -> Result<HttpResponse, Error> {
    let context = Context::new();
    let html = tera
        .render("index.html", &context)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

fn songs(tera: web::Data<tera::Tera>) -> Result<HttpResponse, Error> {
    let context = Context::new();
    let html = tera
        .render("songs.html", &context)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
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

fn artists(tera: web::Data<tera::Tera>) -> Result<HttpResponse, Error> {
    let context = Context::new();
    let html = tera
        .render("artists.html", &context)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
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

fn artist(
    tera: web::Data<tera::Tera>,
    hash: web::Path<u64>,
    collection: web::Data<Collection>,
) -> Result<HttpResponse, Error> {
    let artist = collection.by_artist.get(&hash);
    let html = tera
        .render("artist.html", &artist)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

fn queue(
    tera: web::Data<tera::Tera>,
    queue: web::Data<Arc<Mutex<Vec<Kfile>>>>,
) -> Result<HttpResponse, Error> {
    let _queue = queue.lock().unwrap();
    let queue = _queue.clone();
    drop(_queue);

    let queue = Queue { queue };
    let html = tera
        .render("queue.html", &queue)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

fn add(
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

fn playnow(
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

fn next(worker_sender: web::Data<Sender<WorkerCommand>>) -> HttpResponse {
    let cmd = WorkerCommand::Next;
    worker_sender.send(cmd).unwrap();
    sleep(Duration::from_millis(500));
    HttpResponse::Ok().json(JsonStatus { status: "ok" })
}

fn clear(worker_sender: web::Data<Sender<WorkerCommand>>) -> HttpResponse {
    let cmd = WorkerCommand::ClearQueue;
    worker_sender.send(cmd).unwrap();
    sleep(Duration::from_millis(500));
    HttpResponse::Ok().json(JsonStatus { status: "ok" })
}

fn stop(worker_sender: web::Data<Sender<WorkerCommand>>) -> HttpResponse {
    let cmd = WorkerCommand::Stop;
    worker_sender.send(cmd).unwrap();
    sleep(Duration::from_millis(500));
    HttpResponse::Ok().json(JsonStatus { status: "ok" })
}

fn p404(tera: web::Data<tera::Tera>) -> Result<HttpResponse, Error> {
    let context = HashMap::<String, u64>::new();
    let html = tera
        .render("404.html", &context)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::NotFound()
        .content_type("text/html")
        .body(html))
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

        let mut template_path = CONFIG.data_path.clone();
        template_path.push("templates/**/*");
        let tera = tera::Tera::new(template_path.to_str().unwrap()).unwrap();

        let mut static_path = CONFIG.data_path.clone();
        static_path.push("static");

        App::new()
            .data(collection)
            .data(worker_sender)
            .data(play_queue)
            .data(tera)
            .wrap(middleware::Logger::default()) // enable logger
            .service(web::resource("/").route(web::get().to(index)))
            .service(web::resource("/songs").route(web::get().to(songs)))
            .service(web::resource("/artists").route(web::get().to(artists)))
            .service(web::resource("/artist/{hash}").route(web::get().to(artist)))
            .service(web::resource("/queue").route(web::get().to(queue)))
            .service(web::resource("/api/add").route(web::post().to(add)))
            .service(web::resource("/api/playnow").route(web::post().to(playnow)))
            .service(web::resource("/api/next").route(web::post().to(next)))
            .service(web::resource("/api/clear").route(web::post().to(clear)))
            .service(web::resource("/api/stop").route(web::post().to(stop)))
            .service(web::resource("/api/songs").route(web::get().to(api_songs)))
            .service(web::resource("/api/artists").route(web::get().to(api_artists)))
            .service(actix_files::Files::new("/static", static_path))
            .default_service(
                // 404 for GET request
                web::resource("")
                    .route(web::get().to(p404))
                    // all requests that are not `GET`
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(HttpResponse::MethodNotAllowed),
                    ),
            )
    })
    .bind(addr)?;

    println!("Actix has launched from http://0.0.0.0:{}", port);

    server.run()
}
