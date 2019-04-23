use std::sync::{Arc, Mutex};

use karaoke::collection::Kfile;

lazy_static! {
    pub static ref PLAY_QUEUE: Arc<Mutex<Vec<Kfile>>> = {
        Arc::from(Mutex::from(Vec::new()))
    };
}
