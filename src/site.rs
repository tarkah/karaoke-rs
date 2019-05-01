use crossbeam_channel::Sender;
use karaoke::{
    channel::{WorkerCommand, WORKER_CHANNEL},
    collection::{Collection, Kfile, COLLECTION},
    queue::PLAY_QUEUE,
    CONFIG,
};
use rocket::{
    catch, catchers, get, post,
    request::Form,
    response::{content, NamedFile, Redirect},
    routes, uri, FromForm, State,
};
use rocket_contrib::{
    json,
    json::JsonValue,
    templates::tera::Tera,
};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
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
fn index(tera: State<Tera>) -> content::Html<String> {
    let context = HashMap::<String, u64>::new();
    let html = tera.render("index.html", &context).unwrap();
    content::Html(html)
}

#[get("/songs")]
fn songs(tera: State<Tera>, collection: State<Collection>) -> content::Html<String> {
    let context = collection.inner();
    let html = tera.render("songs.html", &context).unwrap();
    content::Html(html)
}

#[get("/artists")]
fn artists(tera: State<Tera>, collection: State<Collection>) -> content::Html<String> {
    let context = collection.inner();
    let html = tera.render("artists.html", &context).unwrap();
    content::Html(html)
}

#[get("/artist/<hash>")]
fn artist(tera: State<Tera>, hash: u64, collection: State<Collection>) -> content::Html<String> {
    let context = collection.inner();
    let artist = context.by_artist.get(&hash);
    let html = tera.render("artist.html", &artist).unwrap();
    content::Html(html)
}

#[get("/queue")]
fn queue(tera: State<Tera>, queue: State<Arc<Mutex<Vec<Kfile>>>>) -> content::Html<String> {
    let _queue = queue.inner().lock().unwrap();
    let queue = _queue.clone();
    drop(_queue);

    let queue = Queue { queue };
    let html = tera.render("queue.html", &queue).unwrap();
    content::Html(html)
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
    let mut static_path = CONFIG.data_path.clone();
    static_path.push("static");
    NamedFile::open(static_path.join(file)).ok()
}

#[catch(404)]
fn not_found() -> Redirect {
    Redirect::to(uri!(index))
}

fn rocket() -> rocket::Rocket {
    let collection = COLLECTION.clone();
    let worker_sender = WORKER_CHANNEL.0.clone();
    let queue = PLAY_QUEUE.clone();

    let mut template_path = CONFIG.data_path.clone();
    template_path.push("templates/**/*");
    let tera = Tera::new(template_path.to_str().unwrap()).unwrap();

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
        .register(catchers![not_found])
        .manage(collection)
        .manage(worker_sender)
        .manage(queue)
        .manage(tera)
}

pub fn run() {
    rocket().launch();
}
