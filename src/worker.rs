use crossbeam_channel::{select, Receiver, Sender};
use failure::format_err;
use karaoke::{
    channel::{
        LiveCommand, PlayerCommand, WorkerCommand, LIVE_CHANNEL, PLAYER_CHANNEL, WORKER_CHANNEL,
    },
    collection::Kfile,
    log_error,
    queue::PLAY_QUEUE,
    CONFIG,
};
use multiqueue::BroadcastSender;
use std::{
    sync::{Arc, Mutex},
    thread, time,
};

pub fn run() {
    thread::spawn(move || {
        if CONFIG.use_web_player {
            web_worker();
        } else {
            native_worker();
        }
    });
}

fn native_worker() {
    let worker = NativeWorker::new();
    loop {
        select! {
            recv(worker.worker_receiver) -> cmd => {
                match cmd {
                    Ok(cmd) => worker.process_cmd(cmd),
                    Err(e) => log_error(&format_err!("{:?}", e))
                }
            },
            default() => {},
        }
        thread::sleep(time::Duration::from_millis(50));
    }
}

fn web_worker() {
    let (send, recv) = multiqueue::broadcast_queue::<LiveCommand>(5);
    let mut worker = WebWorker::new(send);

    // Start websocket server, will receive commands on Player Receiver
    thread::spawn(move || loop {
        if let Err(e) = karaoke::websocket::start_ws_server(recv.clone()) {
            log_error(&e);
        };
    });

    loop {
        select! {
            recv(worker.worker_receiver) -> cmd => {
                match cmd {
                    Ok(cmd) => worker.process_cmd(cmd),
                    Err(e) => log_error(&format_err!("{:?}", e))
                }
            },
            default() => {},
        }
        thread::sleep(time::Duration::from_millis(50));
    }
}

#[derive(Debug)]
struct NativeWorker {
    worker_receiver: Receiver<WorkerCommand>,
    player_sender: Sender<PlayerCommand>,
    live_sender: Sender<LiveCommand>,
    queue: Arc<Mutex<Vec<Kfile>>>,
}

impl NativeWorker {
    fn new() -> Self {
        let worker_receiver = WORKER_CHANNEL.1.clone();
        let player_sender = PLAYER_CHANNEL.0.clone();
        let live_sender = LIVE_CHANNEL.0.clone();
        let queue = PLAY_QUEUE.clone();
        NativeWorker {
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

struct WebWorker {
    worker_receiver: Receiver<WorkerCommand>,
    live_sender: BroadcastSender<LiveCommand>,
    queue: Arc<Mutex<Vec<Kfile>>>,
}

impl WebWorker {
    fn new(live_sender: BroadcastSender<LiveCommand>) -> Self {
        let worker_receiver = WORKER_CHANNEL.1.clone();
        let queue = PLAY_QUEUE.clone();

        WebWorker {
            worker_receiver,
            live_sender,
            queue,
        }
    }

    fn process_cmd(&mut self, cmd: WorkerCommand) {
        match cmd {
            WorkerCommand::Stop => self.stop(),
            WorkerCommand::Next => self.next(),
            WorkerCommand::PlayNow { kfile } => self.play_now(kfile),
            WorkerCommand::ClearQueue => self.clear_queue(),
            WorkerCommand::AddQueue { kfile } => self.add_queue(kfile),
        }
    }

    fn stop(&mut self) {
        self.clear_queue();

        if let Err(e) = self.live_sender.try_send(LiveCommand::Stop) {
            log_error(&format_err!("{}", e));
        };
    }

    fn next(&mut self) {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            drop(queue);
            return;
        }
        queue.remove(0);
        drop(queue);
        if let Err(e) = self.live_sender.try_send(LiveCommand::Stop) {
            log_error(&format_err!("{}", e));
        };
    }

    fn play_now(&mut self, kfile: Kfile) {
        let mut queue = self.queue.lock().unwrap();
        if !queue.is_empty() {
            queue.remove(0);
        }
        queue.insert(0, kfile);
        drop(queue);
        if let Err(e) = self.live_sender.try_send(LiveCommand::Stop) {
            log_error(&format_err!("{}", e));
        };
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
