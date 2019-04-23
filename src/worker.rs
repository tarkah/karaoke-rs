use std::thread;
use std::time;

use crossbeam_channel::{Sender, Receiver};

use karaoke::collection::Kfile;
use karaoke::channel::{WorkerCommand, PlayerCommand, LiveCommand, WORKER_CHANNEL, PLAYER_CHANNEL, LIVE_CHANNEL};
use karaoke::queue::PLAY_QUEUE;

pub fn run() {
    thread::spawn(move || {
        let worker = Worker::new();
        loop {
            select! {
                recv(worker.worker_receiver) -> cmd => worker.process_cmd(cmd.unwrap()),
                default() => {},
            }
            thread::sleep(time::Duration::from_millis(500));
        }
    });
}

#[derive(Debug)]
struct Worker {
    worker_receiver: Receiver<WorkerCommand>,
    player_sender: Sender<PlayerCommand>,
    live_sender: Sender<LiveCommand>,
}

impl Worker {
    fn new() -> Self {
        let worker_receiver = WORKER_CHANNEL.1.clone();
        let player_sender = PLAYER_CHANNEL.0.clone();
        let live_sender = LIVE_CHANNEL.0.clone();
        Worker { worker_receiver, player_sender, live_sender }
    }

    fn process_cmd(&self, cmd: WorkerCommand) {
        match cmd {
            WorkerCommand::Stop => self.stop(),  
            WorkerCommand::Next => self.next(),  
            WorkerCommand::PlayNow { kfile } => self.play_now(kfile),
            WorkerCommand::ClearQueue => self.clear_queue(),
            WorkerCommand::AddQueue { kfile } => self.add_queue(kfile),
        }
    }

    fn stop(&self) {
        if self.live_sender.is_empty() {
            self.live_sender.send(LiveCommand::Stop).unwrap();
        }
    }

    fn next(&self) {     
        let mut queue = PLAY_QUEUE.lock().unwrap();
        if queue.is_empty() {
            return
        }
        self.live_sender.send(LiveCommand::Stop).unwrap();
        let kfile = queue.remove(0);
        self.player_sender.send(PlayerCommand::Play{ kfile }).unwrap();
    }

    fn play_now(&self, kfile: Kfile) {
        self.live_sender.send(LiveCommand::Stop).unwrap();
        self.player_sender.send(PlayerCommand::Play { kfile }).unwrap();
    }

    fn clear_queue(&self) {
        let mut queue = PLAY_QUEUE.lock().unwrap();
        queue.clear();
    }

    fn add_queue(&self, kfile: Kfile) {
        let mut queue = PLAY_QUEUE.lock().unwrap();
        queue.push(kfile);
    }
}

