#[macro_use] extern crate lazy_static;
#[macro_use] extern crate crossbeam_channel;
extern crate self as karaoke;

mod collection;
mod player;
mod config;
mod channel;

use karaoke::config::{CONFIG};
use karaoke::player::Player;
use karaoke::collection::{COLLECTION};
use karaoke::channel::{LiveCommand, PlayerCommand, LIVE_CHANNEL, PLAYER_CHANNEL};

use std::thread;
use std::time::Duration;


fn main() {    
    test_play();
    //test_collection();
}

#[allow(dead_code)]
fn test_play() {
    let (data_path, allow_overwrite) =
        CONFIG.read(|conf| (conf.data_path.clone(), conf.allow_overwrite.clone())).expect("Read config");
    println!("The current configuration is: {:?} and {}", data_path, allow_overwrite);    

    thread::spawn(move || {
        println!("Initializing Player...");
        let player = Player::new();
        println!("Running Player...");
        player.run();
    });    
    
    for k in COLLECTION.iter() {
        let kfile = k.clone();
        println!("Playing: {:?}", kfile.key);
        PLAYER_CHANNEL.0.send(PlayerCommand::PlayNow { kfile: kfile }).unwrap();        
        std::thread::sleep(Duration::from_millis(1000));
        println!("Stopping...");
        LIVE_CHANNEL.0.send(LiveCommand::Stop).unwrap();
        std::thread::sleep(Duration::from_millis(500));
    } 
}

#[allow(dead_code)]
fn test_collection() {
    if let Err(e) = collection::startup() {
        eprintln!("An error has occurred at: \n{}", e.backtrace());
        ::std::process::exit(1);
    }
}

