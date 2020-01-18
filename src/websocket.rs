use crossbeam_channel::{bounded, select};
use failure::{bail, format_err, Error};
use karaoke::{
    channel::{LiveCommand, WebsocketCommand},
    log_error,
};
use multiqueue::BroadcastReceiver;
use std::{thread, time};
use websocket::{sync::Server, OwnedMessage};

pub fn start_ws_server(receiver: BroadcastReceiver<LiveCommand>) -> Result<(), Error> {
    let mut server = Server::bind("0.0.0.0:9000")?;
    server.set_nonblocking(true)?;
    log::info!("Websocket server has launched from ws://0.0.0.0:9000");

    loop {
        if let Ok(request) = server.accept() {
            let live_receiver = receiver.add_stream();
            thread::spawn(|| {
                //if !request.protocols().contains(&"rust-websocket".to_string()) {
                //    request.reject().unwrap();
                //    let e = format_err!(
                //        "Connection refused: Protocol does not contain 'rust-websocket'"
                //    );
                //    log_error(&e);
                //    bail!(e);
                //}

                let mut client = request.accept().unwrap(); //.use_protocol("rust-websocket").accept().unwrap();

                let ip = client.peer_addr().unwrap();

                log::info!("Connection from {}", ip);

                let message = OwnedMessage::Text("Hello".to_string());
                if let Err(e) = client.send_message(&message) {
                    log_error(&format_err!("Websocket error: {}", e));
                    bail!(e);
                }

                let (mut receiver, mut sender) = client.split().unwrap();

                let (command_sender, command_receiver) = bounded(1);

                thread::spawn(move || {
                    let mut now = time::Instant::now();
                    loop {
                        if let Ok(cmd) = live_receiver.try_recv() {
                            match cmd {
                                LiveCommand::Stop => {
                                    let message = OwnedMessage::Text(String::from("Stop"));
                                    if let Err(e) = sender.send_message(&message) {
                                        log_error(&format_err!("Websocket error: {}", e));
                                        break;
                                    }
                                    log::debug!("Stop command sent to {}", ip);
                                }
                            }
                        }
                        select! {
                            recv(command_receiver) -> cmd => {
                                let cmd = cmd.unwrap();
                                match cmd {
                                    WebsocketCommand::Close => {
                                        break;
                                    }
                                    WebsocketCommand::Ping { data } => {
                                        let message = websocket::message::Message::pong(data);
                                        if let Err(e) = sender.send_message(&message) {
                                            log_error(&format_err!("Websocket error: {}",e));
                                            break;
                                        }
                                    }
                                }
                            }
                            default() => {},
                        }

                        if now.elapsed().as_secs() >= 20 {
                            now = time::Instant::now();
                            let message = OwnedMessage::Ping(String::from("Ping").into_bytes());
                            if let Err(e) = sender.send_message(&message) {
                                log_error(&format_err!("Websocket error: {}", e));
                                break;
                            }
                        }

                        thread::sleep(time::Duration::from_secs(1));
                    }

                    log::debug!("Sender thread disconnected for: {}", ip);
                });

                for message in receiver.incoming_messages() {
                    let message = message.unwrap();

                    match message {
                        OwnedMessage::Close(_) => {
                            log::debug!("Close requested from {}", ip);
                            command_sender.send(WebsocketCommand::Close).unwrap();
                            break;
                        }
                        OwnedMessage::Ping(data) => {
                            log::debug!("Ping received from {}", ip);
                            command_sender
                                .send(WebsocketCommand::Ping { data })
                                .unwrap();
                        }
                        OwnedMessage::Text(text) => {
                            log::debug!("Message received from {}: {}", ip, text);
                        }
                        OwnedMessage::Pong(_) => {
                            log::debug!("Pong received from {}", ip);
                        }
                        _ => {}
                    }
                }
                log::debug!("Receiver thread disconnected for: {}", ip);
                log::info!("Connection ended with {}", ip);
                Ok(())
            });
        };

        let _ = receiver.try_recv();
        thread::sleep(time::Duration::from_millis(100));
    }
}
