use karaoke::collection::Kfile;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};


lazy_static! {
    pub static ref PLAY_QUEUE: Arc<Mutex<Vec<Kfile>>> = { Arc::from(Mutex::from(Vec::new())) };
}
