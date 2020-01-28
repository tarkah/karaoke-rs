use crate::{
    agents::toast::{Msg as ToastAgentMsg, ToastAgent},
    components::toast::{ToastBody, ToastStatus},
    model::{ApiResponse, Artist, Config, DataType, PostSong, RequestParams, Song},
};
use failure::{format_err, Error};
use log::trace;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use yew::{
    agent::{Dispatched, Dispatcher},
    format::{Json, Nothing},
    services::{fetch, FetchService, IntervalService, Task},
    worker::*,
};

#[derive(Serialize, Deserialize)]
pub enum Msg {
    Return {
        who: HandlerId,
        request_type: RequestType,
        response: Response,
    },
    RemoveFetchTasks,
    CreateToast(ToastBody),
}

#[derive(Serialize, Deserialize)]
pub enum Request {
    GetSongs(RequestParams),
    GetArtists(RequestParams),
    GetQueue,
    AddSong(u64),
    PlaySong(u64),
    Stop,
    NextSong,
    ClearQueue,
    Config,
    PlayerNextSong,
    FetchMp3(String),
    FetchCdg(String),
    Ended,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum RequestType {
    GetSongs,
    GetArtists,
    GetQueue,
    AddSong,
    PlaySong,
    Stop,
    NextSong,
    ClearQueue,
    Config,
    PlayerNextSong,
    FetchMp3,
    FetchCdg,
    Ended,
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    Success(ResponseData),
    Error,
}

#[derive(Serialize, Deserialize)]
pub enum ResponseData {
    Songs {
        songs: Vec<Song>,
        total_pages: u32,
    },
    Artists {
        artists: Vec<Artist>,
        total_pages: u32,
    },
    Queue(Vec<Song>),
    Config(Config),
    PlayerNextSong {
        mp3: String,
        cdg: String,
    },
    FileMp3(Vec<u8>),
    FileCdg(Vec<u8>),
    Empty,
}

pub struct ApiAgent {
    link: AgentLink<ApiAgent>,
    fetch_service: FetchService,
    fetch_tasks: Vec<fetch::FetchTask>,
    #[allow(dead_code)]
    job: Box<dyn Task>,
    toast_dispatcher: Dispatcher<ToastAgent>,
}

impl Agent for ApiAgent {
    type Reach = Context;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        let mut interval_service = IntervalService::new();
        let callback = link.callback(|_| Msg::RemoveFetchTasks);
        let handle = interval_service.spawn(Duration::from_millis(1000), callback);

        ApiAgent {
            link,
            fetch_service: FetchService::new(),
            fetch_tasks: vec![],
            job: Box::new(handle),
            toast_dispatcher: ToastAgent::dispatcher(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::RemoveFetchTasks => {
                self.fetch_tasks.retain(|task| task.is_active());
                trace!("# of active fetch tasks: {}", self.fetch_tasks.len());
            }
            Msg::Return {
                who,
                request_type,
                response,
            } => {
                if let Some(toast) = response.toast(request_type) {
                    self.update(Msg::CreateToast(toast));
                }

                self.link.respond(who, response);
            }
            Msg::CreateToast(toast) => {
                self.toast_dispatcher.send(ToastAgentMsg::NewToast(toast));
            }
        }
    }

    fn handle_input(&mut self, msg: Self::Input, who: HandlerId) {
        match msg {
            Request::GetSongs(params) => {
                let fetch_task = self.get_data(who, RequestType::GetSongs, Some(params));
                self.fetch_tasks.push(fetch_task);
            }
            Request::GetArtists(params) => {
                let fetch_task = self.get_data(who, RequestType::GetArtists, Some(params));
                self.fetch_tasks.push(fetch_task);
            }
            Request::GetQueue => {
                let fetch_task = self.get_data(who, RequestType::GetQueue, None);
                self.fetch_tasks.push(fetch_task);
            }
            Request::AddSong(id) => {
                let fetch_task = self.send_command(who, RequestType::AddSong, Some(id));
                self.fetch_tasks.push(fetch_task);
            }
            Request::PlaySong(id) => {
                let fetch_task = self.send_command(who, RequestType::PlaySong, Some(id));
                self.fetch_tasks.push(fetch_task);
            }
            Request::NextSong => {
                let fetch_task = self.send_command(who, RequestType::NextSong, None);
                self.fetch_tasks.push(fetch_task);
            }
            Request::ClearQueue => {
                let fetch_task = self.send_command(who, RequestType::ClearQueue, None);
                self.fetch_tasks.push(fetch_task);
            }
            Request::Stop => {
                let fetch_task = self.send_command(who, RequestType::Stop, None);
                self.fetch_tasks.push(fetch_task);
            }
            Request::Config => {
                let fetch_task = self.get_data(who, RequestType::Config, None);
                self.fetch_tasks.push(fetch_task);
            }
            Request::PlayerNextSong => {
                let fetch_task = self.get_data(who, RequestType::PlayerNextSong, None);
                self.fetch_tasks.push(fetch_task);
            }
            Request::FetchMp3(file_name) => {
                let fetch_task = self.fetch_file(who, RequestType::FetchMp3, file_name);
                self.fetch_tasks.push(fetch_task);
            }
            Request::FetchCdg(file_name) => {
                let fetch_task = self.fetch_file(who, RequestType::FetchCdg, file_name);
                self.fetch_tasks.push(fetch_task);
            }
            Request::Ended => {
                let fetch_task = self.send_command(who, RequestType::Ended, None);
                self.fetch_tasks.push(fetch_task);
            }
        }
    }
}

impl ApiAgent {
    fn get_data(
        &mut self,
        who: HandlerId,
        request_type: RequestType,
        params: Option<RequestParams>,
    ) -> fetch::FetchTask {
        trace!("Fetching data from API");

        let callback = self.link.callback(
            move |response: fetch::Response<Json<Result<ApiResponse, Error>>>| {
                let Json(body) = response.into_body();

                if let Ok(ApiResponse::SuccessGet(data)) = body {
                    let response_data = match data.data {
                        DataType::Songs(songs) => ResponseData::Songs {
                            songs,
                            total_pages: data.total_pages.unwrap_or(0),
                        },
                        DataType::Artists(artists) => ResponseData::Artists {
                            artists,
                            total_pages: data.total_pages.unwrap_or(0),
                        },
                        DataType::Queue(songs) => ResponseData::Queue(songs),
                        DataType::Config(config) => ResponseData::Config(config),
                        DataType::PlayerNextSong { mp3, cdg } => {
                            ResponseData::PlayerNextSong { mp3, cdg }
                        }
                    };

                    return Msg::Return {
                        who,
                        request_type,
                        response: Response::Success(response_data),
                    };
                } else if let Ok(ApiResponse::Error(error)) = body {
                    trace!("Error in API response: {:?}", error.error_message);
                }

                Msg::Return {
                    who,
                    request_type,
                    response: Response::Error,
                }
            },
        );

        let params: String = serde_urlencoded::to_string(&params.unwrap_or_default()).unwrap();
        let request = fetch::Request::get(&format!("/api/{}?{}", request_type.path(), params))
            .body(Nothing)
            .unwrap();

        self.fetch_service.fetch(request, callback)
    }

    fn send_command(
        &mut self,
        who: HandlerId,
        request_type: RequestType,
        song_id: Option<u64>,
    ) -> fetch::FetchTask {
        let callback = self.link.callback(
            move |response: fetch::Response<Json<Result<ApiResponse, Error>>>| {
                let Json(body) = response.into_body();

                if let Ok(ApiResponse::SuccessPost(data)) = body {
                    if data.status == "ok" {
                        trace!("Command successfully posted");
                        return Msg::Return {
                            who,
                            request_type,
                            response: Response::Success(ResponseData::Empty),
                        };
                    } else {
                        trace!("API returned incorrect response data");
                    }
                } else if let Ok(ApiResponse::Error(error)) = body {
                    trace!("Error in API response: {:?}", error.error_message);
                }

                Msg::Return {
                    who,
                    request_type,
                    response: Response::Error,
                }
            },
        );

        let request = if let Some(id) = song_id {
            let body = serde_urlencoded::to_string(PostSong { hash: id })
                .map_err(|_| format_err!("Failed to serialize data"));

            fetch::Request::post(&format!("/api/{}", request_type.path()))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .unwrap()
        } else {
            fetch::Request::post(&format!("/api/{}", request_type.path()))
                .body(Ok(String::from("")))
                .unwrap()
        };

        self.fetch_service.fetch(request, callback)
    }

    fn fetch_file(
        &mut self,
        who: HandlerId,
        request_type: RequestType,
        file_name: String,
    ) -> fetch::FetchTask {
        let callback =
            self.link
                .callback(move |response: fetch::Response<Result<Vec<u8>, Error>>| {
                    let (parts, body) = response.into_parts();

                    if !parts.status.is_success() || body.is_err() {
                        return Msg::Return {
                            who,
                            request_type,
                            response: Response::Error,
                        };
                    }

                    let response_data = match request_type {
                        RequestType::FetchCdg => ResponseData::FileCdg(body.unwrap()),
                        _ => ResponseData::FileMp3(body.unwrap()),
                    };

                    let response = Response::Success(response_data);

                    Msg::Return {
                        who,
                        request_type,
                        response,
                    }
                });

        trace!("Fetching file: {}", file_name);
        let encoded = utf8_percent_encode(&file_name, NON_ALPHANUMERIC).to_string();

        let request = fetch::Request::get(&format!("/songs/{}", encoded))
            .body(Nothing)
            .unwrap();
        self.fetch_service.fetch_binary(request, callback)
    }
}

impl Response {
    fn toast(&self, request_type: RequestType) -> Option<ToastBody> {
        let status = match self {
            Response::Success(..) => ToastStatus::Success,
            Response::Error => ToastStatus::Error,
        };

        let title = match self {
            Response::Success(_) => match request_type {
                RequestType::AddSong => "Added to queue".to_owned(),
                RequestType::PlaySong => "Playing now".to_owned(),
                RequestType::Stop => "Player stopped".to_owned(),
                RequestType::NextSong => "Next song playing".to_owned(),
                RequestType::ClearQueue => "Queue cleared".to_owned(),
                _ => "".to_owned(),
            },
            Response::Error => match request_type {
                RequestType::AddSong => "Failed to add".to_owned(),
                RequestType::PlaySong => "Failed to play".to_owned(),
                RequestType::Stop => "Failed to stop player".to_owned(),
                RequestType::NextSong => "Failed to play next".to_owned(),
                RequestType::ClearQueue => "Failed to clear queue".to_owned(),
                _ => "".to_owned(),
            },
        };

        let toast = ToastBody {
            status,
            title,
            delay: 3000,
        };

        if toast.title != "" {
            Some(toast)
        } else {
            None
        }
    }
}

impl RequestType {
    fn path(&self) -> &str {
        match self {
            RequestType::AddSong => "add",
            RequestType::PlaySong => "playnow",
            RequestType::NextSong => "next",
            RequestType::ClearQueue => "clear",
            RequestType::Stop => "stop",
            RequestType::GetSongs => "songs",
            RequestType::GetArtists => "artists",
            RequestType::GetQueue => "queue",
            RequestType::Config => "config",
            RequestType::PlayerNextSong => "player/next",
            RequestType::Ended => "player/ended",
            _ => "",
        }
    }
}
