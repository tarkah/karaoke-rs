use super::api;
use failure::Error;
use gloo_events::EventListener;
use image::{GenericImage, RgbaImage};
use js_sys::Uint8ClampedArray;
use log::trace;
use serde::{Deserialize, Serialize};
use std::{f32::consts, io::Cursor, time::Duration};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, AudioNode};
use yew::{
    format::Json,
    services::{
        render::{RenderService, RenderTask},
        timeout::{TimeoutService, TimeoutTask},
        websocket::{WebSocketService, WebSocketStatus, WebSocketTask},
    },
    worker::*,
};

#[derive(Serialize, Deserialize)]
pub struct WsMessage {
    pub command: String,
}

pub enum Msg {
    MainLoop,
    PlayingLoop,
    NotPlayingLoop,
    Stop,
    Ended,
    GetSong,
    FetchMp3(String),
    FetchCdg(String),
    DecodeMp3,
    PlayMp3(AudioBuffer),
    StartCdgPlayer,
    ApiResponse(api::Response),
    WsReceived(Json<Result<WsMessage, Error>>),
    WsStatus(WebSocketStatus),
}

#[derive(Serialize, Deserialize)]
pub enum Request {
    Port(u16),
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    RenderFrame {
        cdg_frame: Vec<u8>,
        background: (f32, f32, f32, f32),
    },
    ClearCanvas,
}

#[derive(PartialEq)]
enum FileStatus {
    None,
    Fetching,
    Fetched(Vec<u8>),
}

pub struct PlayerAgent {
    #[allow(dead_code)]
    link: AgentLink<PlayerAgent>,
    api_agent: Box<dyn Bridge<api::ApiAgent>>,
    bridged_component: Option<HandlerId>,
    #[allow(dead_code)]
    ws_task: Option<WebSocketTask>,
    timeout_service: TimeoutService,
    timeout_task: Option<TimeoutTask>,
    render_service: RenderService,
    render_task: Option<RenderTask>,
    audio_context: Option<AudioContext>,
    buffer_source_node: Option<AudioBufferSourceNode>,
    buffer_source_node_onended: Option<EventListener>,
    playing: bool,
    song_start_time: f64,
    mp3: FileStatus,
    cdg: FileStatus,
    cdg_player: Option<Cdg>,
    last_sector: f64,
}

impl Agent for PlayerAgent {
    type Reach = Job;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        let callback = link.callback(Msg::ApiResponse);
        let api_agent = api::ApiAgent::bridge(callback);

        PlayerAgent {
            link,
            api_agent,
            bridged_component: None,
            ws_task: None,
            timeout_service: TimeoutService::new(),
            timeout_task: None,
            render_service: RenderService::new(),
            render_task: None,
            audio_context: None,
            buffer_source_node: None,
            buffer_source_node_onended: None,
            playing: false,
            song_start_time: 0.0,
            mp3: FileStatus::None,
            cdg: FileStatus::None,
            cdg_player: None,
            last_sector: 0.0,
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.bridged_component = Some(id);
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::MainLoop => {
                trace!("Main loop...");

                if self.playing {
                    self.update(Msg::PlayingLoop);
                } else {
                    self.update(Msg::NotPlayingLoop);
                }
            }
            Msg::PlayingLoop => {
                self.playing_loop();
            }
            Msg::NotPlayingLoop => {
                self.not_playing_loop();
            }
            Msg::Stop => {
                self.render_task = None;

                trace!("Stopping player...");
                self.buffer_source_node_onended = None;
                let _ = self.buffer_source_node.as_ref().unwrap().disconnect();
                self.buffer_source_node = None;

                self.mp3 = FileStatus::None;
                self.cdg = FileStatus::None;
                self.cdg_player = None;
                self.last_sector = 0.0;

                self.playing = false;

                self.link
                    .respond(self.bridged_component.unwrap(), Response::ClearCanvas);

                self.timeout_task = Some(self.timeout_service.spawn(
                    Duration::from_millis(3000),
                    self.link.callback(|_| Msg::MainLoop),
                ));
            }
            Msg::Ended => {
                trace!("Song ended...");
                self.api_agent.send(api::Request::Ended);
                self.update(Msg::Stop);
            }
            Msg::GetSong => {
                self.api_agent.send(api::Request::PlayerNextSong);
            }
            Msg::FetchMp3(file_name) => {
                self.api_agent.send(api::Request::FetchMp3(file_name));
                self.mp3 = FileStatus::Fetching;
            }
            Msg::FetchCdg(file_name) => {
                self.api_agent.send(api::Request::FetchCdg(file_name));
                self.cdg = FileStatus::Fetching;
            }
            Msg::DecodeMp3 => {
                if let FileStatus::Fetched(bytes) = &self.mp3 {
                    trace!("Decoding audio data...");

                    let clamped_array = Uint8ClampedArray::from(&bytes[..]);
                    let array_buffer = clamped_array.buffer();

                    let audio_context = get_audio_context().unwrap();

                    let promise = audio_context.decode_audio_data(&array_buffer).unwrap();

                    let callback = self.link.callback(Msg::PlayMp3);

                    spawn_local(async move {
                        let future = JsFuture::from(promise);
                        let decoded: AudioBuffer = future.await.unwrap().unchecked_into();
                        trace!("Audio data decoded into Audio Buffer");

                        callback.emit(decoded);
                    });
                }
            }
            Msg::PlayMp3(audio) => {
                self.buffer_source_node
                    .as_ref()
                    .unwrap()
                    .set_buffer(Some(&audio));

                let source_node: &AudioNode = self.buffer_source_node.as_ref().unwrap();
                let destination = self.audio_context.as_ref().unwrap().destination();
                let destination_node: &AudioNode = destination.as_ref();
                let connect_result = source_node.connect_with_audio_node(destination_node);

                let play_result = self.buffer_source_node.as_ref().unwrap().start();

                if play_result.is_ok() && connect_result.is_ok() {
                    self.playing = true;
                    self.song_start_time = self.audio_context.as_ref().unwrap().current_time();
                    trace!("Audio is playing");
                }
            }
            Msg::StartCdgPlayer => {
                if let FileStatus::Fetched(bytes) = &self.cdg {
                    let cdg_player = Cdg::new(bytes.clone());
                    self.cdg_player = Some(cdg_player);
                }
            }
            Msg::ApiResponse(response) => match response {
                api::Response::Success(api::ResponseData::PlayerNextSong { mp3, cdg }) => {
                    self.update(Msg::FetchMp3(mp3));
                    self.update(Msg::FetchCdg(cdg));
                }
                api::Response::Success(api::ResponseData::FileMp3(bytes)) => {
                    log::trace!("Got mp3, is {} bytes", bytes.len());
                    self.mp3 = FileStatus::Fetched(bytes);
                    self.update(Msg::DecodeMp3);
                }
                api::Response::Success(api::ResponseData::FileCdg(bytes)) => {
                    log::trace!("Got cdg, is {} bytes", bytes.len());
                    self.cdg = FileStatus::Fetched(bytes);
                    self.update(Msg::StartCdgPlayer);
                }
                _ => {}
            },
            Msg::WsReceived(Json(response)) => match response {
                Ok(data) => {
                    log::trace!("Websocket Received command: {}", data.command);
                    match data.command.as_str() {
                        "stop" => {
                            self.update(Msg::Stop);
                        }
                        "hello" => {
                            self.update(Msg::MainLoop);
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    log::trace!("Websocket Error: {}", e);
                }
            },
            Msg::WsStatus(status) => match status {
                WebSocketStatus::Error => log::trace!("Websocket failure"),
                WebSocketStatus::Closed => log::trace!("Websocket closed"),
                WebSocketStatus::Opened => log::trace!("Websocket connection established"),
            },
        }
    }

    fn handle_input(&mut self, msg: Self::Input, _: HandlerId) {
        match msg {
            Request::Port(port) => {
                let mut ws_service = WebSocketService::new();
                let callback = self.link.callback(Msg::WsReceived);
                let notification = self.link.callback(Msg::WsStatus);
                let ws_task = ws_service
                    .connect(&get_ws_host(port), callback, notification)
                    .ok();

                self.ws_task = ws_task;
            }
        }
    }
}

impl PlayerAgent {
    fn playing_loop(&mut self) {
        let time_played =
            self.audio_context.as_ref().unwrap().current_time() - self.song_start_time;

        let calc_sector = (time_played / 0.013_333_333).floor();

        if calc_sector >= 0.0 {
            let sectors_since = calc_sector - self.last_sector;

            let cdg_frame = self.cdg_player.as_mut().unwrap().next_frame(sectors_since);
            let background = self.cdg_player.as_mut().unwrap().rainbow_cycle();
            let response = Response::RenderFrame {
                cdg_frame,
                background,
            };

            self.last_sector = calc_sector;

            self.link.respond(self.bridged_component.unwrap(), response);
        }

        self.render_task = Some(
            self.render_service
                .request_animation_frame(self.link.callback(|_| Msg::PlayingLoop)),
        );
    }

    fn not_playing_loop(&mut self) {
        if self.audio_context.is_none() {
            self.audio_context = get_audio_context();
            trace!("Got audio context");
        }

        if self.buffer_source_node.is_none() && self.audio_context.is_some() {
            self.buffer_source_node = get_buffer_source(&self.audio_context.as_ref().unwrap());

            let callback = self.link.callback(|_| Msg::Ended);
            let onended = EventListener::new(
                &self.buffer_source_node.as_ref().unwrap().as_ref(),
                "ended",
                move |_| {
                    callback.emit(());
                },
            );
            self.buffer_source_node_onended = Some(onended);

            trace!("Got buffer source");
        }

        if self.cdg == FileStatus::None && self.mp3 == FileStatus::None {
            self.update(Msg::GetSong);
            trace!("Getting next song...");
        }

        self.timeout_task = Some(self.timeout_service.spawn(
            Duration::from_millis(1000),
            self.link.callback(|_| Msg::MainLoop),
        ));
    }
}

fn get_ws_host(port: u16) -> String {
    let window = web_sys::window().unwrap();
    let location = window.location();

    let hostname = location.hostname().unwrap();

    format!("ws://{}:{}", hostname, port)
}

fn get_audio_context() -> Option<AudioContext> {
    AudioContext::new().ok()
}

fn get_buffer_source(audio_context: &AudioContext) -> Option<AudioBufferSourceNode> {
    audio_context.create_buffer_source().ok()
}

#[wasm_bindgen]
pub struct Cdg {
    scsi: cdg::SubchannelStreamIter<Cursor<Vec<u8>>>,
    interpreter: cdg_renderer::CdgInterpreter,
    image: RgbaImage,
    i: f32,
}

impl Cdg {
    pub fn new(cdg: Vec<u8>) -> Cdg {
        let scsi = cdg::SubchannelStreamIter::new(Cursor::new(cdg));
        let interpreter = cdg_renderer::CdgInterpreter::new();
        let image = RgbaImage::new(300, 216);

        Cdg {
            scsi,
            interpreter,
            image,
            i: 0.0,
        }
    }

    pub fn next_frame(&mut self, sectors_since: f64) -> Vec<u8> {
        for _ in 0..sectors_since as usize {
            let sector = self.scsi.next();

            //Break the loop once no more sectors exist to render
            if let Some(s) = sector {
                for cmd in s {
                    self.interpreter.handle_cmd(cmd);
                }
            }
        }

        self.image.copy_from(&self.interpreter, 0, 0);
        self.image.clone().into_vec()
    }

    //Sine wave formula for rainbow cycling background color
    pub fn rainbow_cycle(&mut self) -> (f32, f32, f32, f32) {
        self.i = if (self.i + 1.0) % 4096.0 == 0.0 {
            0.0
        } else {
            self.i + 1.0
        };
        let r = ((consts::PI / 4096.0 * 2.0 * self.i + 0.0 * consts::PI / 3.0).sin() * 127.0)
            .floor()
            + 128.0;
        let g = ((consts::PI / 4096.0 * 2.0 * self.i + 4.0 * consts::PI / 3.0).sin() * 127.0)
            .floor()
            + 128.0;
        let b = ((consts::PI / 4096.0 * 2.0 * self.i + 8.0 * consts::PI / 3.0).sin() * 127.0)
            .floor()
            + 128.0;
        let a = 1.0;

        (r, g, b, a)
    }
}
