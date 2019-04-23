#[macro_use] extern crate lazy_static;
#[macro_use] extern crate crossbeam_channel;
extern crate self as karaoke;

mod collection;
mod player;
mod config;
mod channel;
mod worker;
mod queue;

use karaoke::config::{CONFIG};
use karaoke::player::Player;
use karaoke::collection::{COLLECTION, Kfile};
use karaoke::channel::{WorkerCommand, LiveCommand, PlayerCommand, WORKER_CHANNEL, LIVE_CHANNEL, PLAYER_CHANNEL};
use karaoke::queue::PLAY_QUEUE;

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

    player::run();
    worker::run();
    
    let mut q_len = 0;
    for k in COLLECTION.iter() {
        let kfile = k.clone();
        let mut queue = PLAY_QUEUE.lock().unwrap();
        queue.push(kfile);   
        q_len += 1     
    }

    for x in 0..q_len {
        if x==0 {
            let mut kfile: Kfile;
            { kfile = PLAY_QUEUE.lock().unwrap().remove(0); }
            WORKER_CHANNEL.0.send(WorkerCommand::PlayNow { kfile }).unwrap(); 
            std::thread::sleep(Duration::from_millis(2000));  
        } else if x<4 {
            WORKER_CHANNEL.0.send(WorkerCommand::Next).unwrap();        
            std::thread::sleep(Duration::from_millis(2000));
        } else if x==4 {
            WORKER_CHANNEL.0.send(WorkerCommand::ClearQueue).unwrap();
            WORKER_CHANNEL.0.send(WorkerCommand::Next).unwrap();
            std::thread::sleep(Duration::from_millis(2000));
        } else {
            WORKER_CHANNEL.0.send(WorkerCommand::Next).unwrap();
            std::thread::sleep(Duration::from_millis(2000));
            WORKER_CHANNEL.0.send(WorkerCommand::Stop).unwrap();
        }
    }    
}

#[allow(dead_code)]
fn test_collection() {
    if let Err(e) = collection::startup() {
        eprintln!("An error has occurred at: \n{}", e.backtrace());
        ::std::process::exit(1);
    }
}

