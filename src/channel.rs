use crossbeam_channel::{bounded, Receiver, Sender};
use karaoke::collection::Kfile;
use lazy_static::lazy_static;


lazy_static! {
    pub static ref WORKER_CHANNEL: (Sender<WorkerCommand>, Receiver<WorkerCommand>) = {
        let (player_send, player_receive) = bounded(1);
        (player_send, player_receive)
    };
    pub static ref PLAYER_CHANNEL: (Sender<PlayerCommand>, Receiver<PlayerCommand>) = {
        let (player_send, player_receive) = bounded(1);
        (player_send, player_receive)
    };
    pub static ref LIVE_CHANNEL: (Sender<LiveCommand>, Receiver<LiveCommand>) = {
        let (live_send, live_receive) = bounded(1);
        (live_send, live_receive)
    };
}

#[derive(Eq, PartialEq, Debug)]
pub enum WorkerCommand {
    Stop,
    Next,
    PlayNow { kfile: Kfile },
    ClearQueue,
    AddQueue { kfile: Kfile },
}

#[derive(Eq, PartialEq, Debug)]
pub enum PlayerCommand {
    Play { kfile: Kfile },
}

#[derive(Eq, PartialEq, Debug)]
pub enum LiveCommand {
    Stop, //
}
