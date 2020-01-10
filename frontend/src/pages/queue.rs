use crate::model::{ApiResponse, DataType, Song};
use failure::Error;
use log::trace;
use std::time::Duration;
use yew::{
    format::{Json, Nothing},
    prelude::*,
    services::{
        fetch::{FetchTask, Request, Response},
        FetchService, IntervalService, Task,
    },
};

pub enum Msg {
    Clear,
    Stop,
    Next,
    FetchQueue,
    StoreQueue(Vec<Song>),
    Noop,
}

pub struct QueuePage {
    fetch_service: FetchService,
    fetch_task: Option<FetchTask>,
    link: ComponentLink<QueuePage>,
    queue: Vec<Song>,
    #[allow(dead_code)]
    job: Box<dyn Task>,
}

impl Component for QueuePage {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let callback = link.callback(|_| Msg::FetchQueue);
        let mut interval = IntervalService::new();
        let handle = interval.spawn(Duration::from_millis(1000), callback);

        QueuePage {
            fetch_service: FetchService::new(),
            fetch_task: None,
            link,
            queue: vec![],
            job: Box::new(handle),
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.link.send_message(Msg::FetchQueue);
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Clear => {
                trace!("Clearing queue");
                let fetch_task = self.send_command("clear");
                self.fetch_task = Some(fetch_task);
            }
            Msg::Stop => {
                trace!("Stopping player");
                let fetch_task = self.send_command("stop");
                self.fetch_task = Some(fetch_task);
            }
            Msg::Next => {
                trace!("Requesting next song");
                let fetch_task = self.send_command("next");
                self.fetch_task = Some(fetch_task);
            }
            Msg::FetchQueue => {
                let fetch_task = self.fetch_queue();
                self.fetch_task = Some(fetch_task);
            }
            Msg::StoreQueue(queue) => {
                self.queue = queue;
                return true;
            }
            Msg::Noop => {}
        }
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <div class="queue__actions">
                    <button class="button"
                        role="button" aria-pressed="true" onclick=self.link.callback(|_| Msg::Clear)>{ "Clear Queue" }</button>
                    <button class="button"
                        role="button" aria-pressed="true" onclick=self.link.callback(|_| Msg::Next)>{ "Next Song" }</button>
                    <button class="button"
                        role="button" aria-pressed="true" onclick=self.link.callback(|_| Msg::Stop)>{ "Stop" }</button>
                </div>
                { self.view_table() }
            </div>
        }
    }
}

impl QueuePage {
    fn view_row(&self, idx: usize, song: Song) -> Html {
        html! {
            <tr>
                <th scope="row" class="text-center">{ idx + 1 }</th>
                <td>{ song.name }</td>
                <td class="text-center">{ song.artist_name }</td>
            </tr>
        }
    }

    fn view_table(&self) -> Html {
        html! {
            <div>
                <div class="justify-content-center">
                    <table class="table table-striped table-bordered">
                        <thead>
                            <tr>
                                <th scope="col" class="text-center">{ "#" }</th>
                                <th scope="col">{ "Song" }</th>
                                <th scope="col" class="text-center">{ "Artist" }</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                for self.queue.iter().enumerate().map(|(idx, song)| {
                                    self.view_row(idx, song.clone())
                                })
                            }
                        </tbody>
                    </table>
                </div>
            </div>
        }
    }

    fn fetch_queue(&mut self) -> FetchTask {
        let callback = self.link.callback(
            move |response: Response<Json<Result<ApiResponse, Error>>>| {
                let Json(body) = response.into_body();

                if let Ok(ApiResponse::SuccessGet(data)) = body {
                    if let DataType::Queue(queue) = data.data {
                        Msg::StoreQueue(queue)
                    } else {
                        trace!("API returned incorrect response data");
                        Msg::Noop
                    }
                } else if let Ok(ApiResponse::Error(error)) = body {
                    trace!("Error in API response: {:?}", error.error_message);
                    Msg::Noop
                } else {
                    trace!("Error in API response");
                    Msg::Noop
                }
            },
        );

        let request = Request::get("/api/queue").body(Nothing).unwrap();
        self.fetch_service.fetch(request, callback)
    }

    fn send_command(&mut self, command: &str) -> FetchTask {
        trace!("Sending command to API: {}", command);

        let callback = self.link.callback(
            move |response: Response<Json<Result<ApiResponse, Error>>>| {
                let (meta, _) = response.into_parts();

                if !meta.status.is_success() {
                    trace!("Error submitting command");
                }

                Msg::FetchQueue
            },
        );

        let request = Request::post(&format!("/api/{}", command))
            .body(Nothing)
            .unwrap();
        self.fetch_service.fetch(request, callback)
    }
}
