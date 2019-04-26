use std::{
    sync::{Arc, Mutex},
    thread, time,
};

use crossbeam_channel::{Receiver, Sender};

use karaoke::{
    channel::{
        LiveCommand, PlayerCommand, WorkerCommand, LIVE_CHANNEL, PLAYER_CHANNEL, WORKER_CHANNEL,
    },
    collection::Kfile,
    queue::PLAY_QUEUE,
};

pub fn run() {
    thread::spawn(move || {
        let worker = Worker::new();
        loop {
            select! {
                recv(worker.worker_receiver) -> cmd => worker.process_cmd(cmd.unwrap()),
                default() => {},
            }
            thread::sleep(time::Duration::from_millis(50));
        }
    });
}

#[derive(Debug)]
struct Worker {
    worker_receiver: Receiver<WorkerCommand>,
    player_sender: Sender<PlayerCommand>,
    live_sender: Sender<LiveCommand>,
    queue: Arc<Mutex<Vec<Kfile>>>,
}

impl Worker {
    fn new() -> Self {
        let worker_receiver = WORKER_CHANNEL.1.clone();
        let player_sender = PLAYER_CHANNEL.0.clone();
        let live_sender = LIVE_CHANNEL.0.clone();
        let queue = PLAY_QUEUE.clone();
        Worker {
            worker_receiver,
            player_sender,
            live_sender,
            queue,
        }
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
        self.clear_queue();

        if self.live_sender.is_empty() {
            self.live_sender.send(LiveCommand::Stop).unwrap();
        }
    }

    fn next(&self) {
        let queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            drop(queue);
            return;
        }
        drop(queue);
        self.live_sender.send(LiveCommand::Stop).unwrap();
    }

    fn play_now(&self, kfile: Kfile) {
        self.live_sender.send(LiveCommand::Stop).unwrap();
        self.player_sender
            .send(PlayerCommand::Play { kfile })
            .unwrap();
    }

    fn clear_queue(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.clear();
        drop(queue);
    }

    fn add_queue(&self, kfile: Kfile) {
        let mut queue = self.queue.lock().unwrap();
        queue.push(kfile);
        drop(queue);
    }
}
