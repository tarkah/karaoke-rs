extern crate self as karaoke;

use clap::{App, Arg};
use env_logger::Env;
use failure::{bail, format_err, Error, ResultExt};
use karaoke::{
    collection::COLLECTION,
    config::{load_config, Config},
};
use lazy_static::lazy_static;
use std::{fs::metadata, path::PathBuf};

mod channel;
mod collection;
mod config;
mod embed;
mod player;
mod queue;
mod site;
mod websocket;
mod worker;

lazy_static! {
    pub static ref CONFIG: Config = {
        let config = get_config();
        match config {
            Ok(c) => c,
            Err(e) => {
                log_error(&e);
                std::process::exit(1);
            }
        }
    };
}

fn main() {
    if let Err(e) = run() {
        log_error(&e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    env_logger::from_env(Env::default().default_filter_or("karaoke_rs=info")).init();

    lazy_static::initialize(&CONFIG);
    lazy_static::initialize(&COLLECTION);
    karaoke::embed::unload_files();
    if !&CONFIG.use_web_player {
        karaoke::player::run();
    }
    karaoke::worker::run();
    karaoke::site::run()?;
    Ok(())
}

fn get_config() -> Result<Config, failure::Error> {
    let matches = App::new("karoake-rs")
        .version(env!("CARGO_PKG_VERSION"))
        .author("tarkah <admin@tarkah.dev>")
        .about("A simple, network enabled karaoke player in Rust")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("songs")
                .short("s")
                .long("songs")
                .value_name("DIR")
                .help("Sets a custom song directory")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("data")
                .short("d")
                .long("data")
                .value_name("DIR")
                .help("Sets a custom data directory")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("refresh-collection")
                .short("r")
                .long("refresh-collection")
                .value_name("BOOL")
                .help("Specify if collection should be refreshed on startup")
                .takes_value(true)
                .possible_values(&["true", "false"]),
        )
        .arg(
            Arg::with_name("use-web-player")
                .short("w")
                .long("use-web-player")
                .help("Use web player instead of native player"),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .help("Specify website port")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port-ws")
                .long("port-ws")
                .value_name("PORT_WS")
                .help("Specify a websocket port when using the web player feature")
                .takes_value(true),
        )
        .get_matches();

    let config_path: Option<PathBuf>;
    let song_path: Option<PathBuf>;
    let data_path: Option<PathBuf>;

    //Return each path if valid, panic if not
    let _config_path = matches.value_of("config");
    config_path = match _config_path {
        Some(path) => validate_file(path)?,
        None => None,
    };
    let _song_path = matches.value_of("songs");
    song_path = match _song_path {
        Some(path) => validate_dir(path)?,
        None => None,
    };
    let _data_path = matches.value_of("data");
    data_path = match _data_path {
        Some(path) => validate_dir(path)?,
        None => None,
    };
    let refresh_collection = if matches.is_present("refresh-collection") {
        Some(
            matches
                .value_of("refresh-collection")
                .unwrap()
                .parse::<bool>()
                .unwrap(),
        )
    } else {
        None
    };
    let use_web_player = if matches.is_present("use-web-player") {
        Some(true)
    } else {
        None
    };
    let port = if matches.is_present("port") {
        Some(matches.value_of("port").unwrap().parse::<u16>().unwrap())
    } else {
        None
    };
    let port_ws = if matches.is_present("port-ws") {
        Some(matches.value_of("port-ws").unwrap().parse::<u16>().unwrap())
    } else {
        None
    };

    //Load config file from config_path, override config with supplied Args, if applicable
    load_config(
        config_path,
        song_path,
        data_path,
        refresh_collection,
        use_web_player,
        port,
        port_ws,
    )
}

fn validate_file(path: &str) -> Result<Option<PathBuf>, Error> {
    let meta = metadata(path).context(format_err!("File doesn't exist: {:?}", path))?;
    let permissions = meta.permissions();
    if !meta.is_file() {
        bail!("File supplied as argument is not valid: {}", path)
    }
    if permissions.readonly() {
        bail!("Do you have permissions for: {}", path)
    }
    Ok(Some(PathBuf::from(path)))
}

fn validate_dir(path: &str) -> Result<Option<PathBuf>, Error> {
    let meta = metadata(path).context(format_err!("Directory doesn't exist: {:?}", path))?;
    let permissions = meta.permissions();
    if !meta.is_dir() {
        bail!("Dir supplied as argument is not valid: {}", path)
    }
    if permissions.readonly() {
        bail!("Do you have permissions for: {}", path)
    }
    Ok(Some(PathBuf::from(path)))
}

/// Log any errors and causes
pub fn log_error(e: &Error) {
    log::error!("{}", e);
    for cause in e.iter_causes() {
        log::error!("Caused by: {}", cause);
    }
}
