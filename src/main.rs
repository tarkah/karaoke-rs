#[macro_use] extern crate lazy_static;

mod collection;
mod player;
mod config;

use config::{CONFIG};
use player::play;
use collection::Kfile;

fn main() {    
    test_play();
    //test_collection();    
}

#[allow(dead_code)]
fn test_play() {
    let (data_path, allow_overwrite) =
        CONFIG.read(|conf| (conf.data_path.clone(), conf.allow_overwrite.clone())).expect("Read config");
    println!("The current configuration is: {:?} and {}", data_path, allow_overwrite);

    let collection = collection::startup().unwrap();    
    let mut kfile = Kfile::default();
    for k in collection.iter() {
        kfile = k.clone();
        break
    }
    play(kfile).unwrap(); 
}

#[allow(dead_code)]
fn test_collection() {
    if let Err(e) = collection::startup() {
        eprintln!("An error has occurred at: \n{}", e.backtrace());
        ::std::process::exit(1);
    }
}

