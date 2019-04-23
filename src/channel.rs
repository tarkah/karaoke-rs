use karaoke::collection::Kfile;

use crossbeam_channel::{bounded, Sender, Receiver};

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
    Stop,  //Upon receive, sends LiveCommand::Stop to Live Channel
    Next,  //Upon receive, sends LiveCommand::Stop to Live Channel, gets next song in queue,
           // then sends PlayerCommand::Play to Player Channel. If queue empty, doesn't send Play.
    PlayNow { kfile: Kfile },   //Upon receive, sends LiveCommand::Stop to Live Channel,
                                // send PlayerCommand::Play to Player Channel with applicable song
    ClearQueue,  //Upon receive, clears play queue
    AddQueue { kfile: Kfile },  //Upon receive, adds song to play queue
}

#[derive(Eq, PartialEq, Debug)]
pub enum PlayerCommand {
    Play { kfile: Kfile },  //Triggered by ManagerCommand::{PlayNow, Next}
}

#[derive(Eq, PartialEq, Debug)]
pub enum LiveCommand {
    Stop,   //
}