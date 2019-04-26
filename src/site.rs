use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use serde_derive::{Deserialize, Serialize};

use rocket::{
    request::Form,
    response::{NamedFile, Redirect},
    State,
};
use rocket_contrib::{json::JsonValue, templates::Template};

use crossbeam_channel::Sender;

use karaoke::{
    channel::{WorkerCommand, WORKER_CHANNEL},
    collection::{Collection, Kfile, COLLECTION},
    queue::PLAY_QUEUE,
};

#[derive(FromForm)]
struct Song {
    hash: u64,
}

#[derive(Serialize, Deserialize)]
struct Queue {
    queue: Vec<Kfile>,
}

#[get("/")]
fn index() -> Template {
    let context = HashMap::<String, u64>::new();
    Template::render("index", &context)
}

#[get("/songs")]
fn songs(collection: State<Collection>) -> Template {
    let context = collection.inner();
    Template::render("songs", &context)
}

#[get("/artists")]
fn artists(collection: State<Collection>) -> Template {
    let context = collection.inner();

    Template::render("artists", &context)
}

#[get("/artist/<hash>")]
fn artist(hash: u64, collection: State<Collection>) -> Template {
    let context = collection.inner();
    let artist = context.by_artist.get(&hash);

    Template::render("artist", &artist)
}

#[get("/queue")]
fn queue(queue: State<Arc<Mutex<Vec<Kfile>>>>) -> Template {
    let _queue = queue.inner().lock().unwrap();
    let queue = _queue.clone();
    drop(_queue);

    let queue = Queue { queue };

    Template::render("queue", &queue)
}

#[post("/api/add", data = "<form>")]
fn add(
    form: Form<Song>,
    collection: State<Collection>,
    worker_sender: State<Sender<WorkerCommand>>,
) -> JsonValue {
    let hash = form.hash;
    let collection = collection.inner();
    let kfile = collection.by_song.get(&hash).unwrap().clone();
    let cmd = WorkerCommand::AddQueue { kfile };
    worker_sender.send(cmd).unwrap();

    json!({ "status": "ok" })
}

#[post("/api/playnow", data = "<form>")]
fn playnow(
    form: Form<Song>,
    collection: State<Collection>,
    worker_sender: State<Sender<WorkerCommand>>,
) -> JsonValue {
    let hash = form.hash;
    let collection = collection.inner();
    let kfile = collection.by_song.get(&hash).unwrap().clone();
    let cmd = WorkerCommand::PlayNow { kfile };
    worker_sender.send(cmd).unwrap();

    json!({ "status": "ok" })
}

#[post("/api/next")]
fn next(worker_sender: State<Sender<WorkerCommand>>) -> JsonValue {
    let cmd = WorkerCommand::Next;
    worker_sender.send(cmd).unwrap();

    sleep(Duration::from_millis(500));
    json!({ "status": "ok" })
}

#[post("/api/clear")]
fn clear(worker_sender: State<Sender<WorkerCommand>>) -> JsonValue {
    let cmd = WorkerCommand::ClearQueue;
    worker_sender.send(cmd).unwrap();

    sleep(Duration::from_millis(500));
    json!({ "status": "ok" })
}

#[post("/api/stop")]
fn stop(worker_sender: State<Sender<WorkerCommand>>) -> JsonValue {
    let cmd = WorkerCommand::Stop;
    worker_sender.send(cmd).unwrap();

    sleep(Duration::from_millis(500));
    json!({ "status": "ok" })
}

#[get("/static/<file..>", rank = 1)]
fn static_files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static").join(file)).ok()
}

#[catch(404)]
fn not_found() -> Redirect {
    Redirect::to(uri!(index))
}

fn rocket() -> rocket::Rocket {
    let collection = COLLECTION.clone();
    let worker_sender = WORKER_CHANNEL.0.clone();
    let queue = PLAY_QUEUE.clone();

    rocket::ignite()
        .mount(
            "/",
            routes![
                index,
                songs,
                artists,
                artist,
                queue,
                add,
                next,
                playnow,
                clear,
                stop,
                static_files
            ],
        )
        .attach(Template::fairing())
        .register(catchers![not_found])
        .manage(collection)
        .manage(worker_sender)
        .manage(queue)
}

pub fn run() {
    rocket().launch();
}
