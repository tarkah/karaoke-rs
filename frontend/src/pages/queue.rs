use crate::{agents::api, model::Song};
use log::trace;
use std::time::Duration;
use yew::{
    prelude::*,
    services::{IntervalService, Task},
};

pub enum Msg {
    Clear,
    Stop,
    Next,
    GetQueue,
    ApiResponse(api::Response),
}

pub struct QueuePage {
    link: ComponentLink<QueuePage>,
    api_agent: Box<dyn Bridge<api::ApiAgent>>,
    queue: Vec<Song>,
    #[allow(dead_code)]
    job: Box<dyn Task>,
}

impl Component for QueuePage {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let api_agent = api::ApiAgent::bridge(link.callback(Msg::ApiResponse));

        let callback = link.callback(|_| Msg::GetQueue);
        let mut interval = IntervalService::new();
        let handle = interval.spawn(Duration::from_millis(1000), callback);

        QueuePage {
            link,
            api_agent,
            queue: vec![],
            job: Box::new(handle),
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.link.send_message(Msg::GetQueue);
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Clear => {
                trace!("Clearing queue");
                self.api_agent.send(api::Request::ClearQueue);
                self.update(Msg::GetQueue);
            }
            Msg::Stop => {
                trace!("Stopping player");
                self.api_agent.send(api::Request::Stop);
                self.update(Msg::GetQueue);
            }
            Msg::Next => {
                trace!("Requesting next song");
                self.api_agent.send(api::Request::NextSong);
                self.update(Msg::GetQueue);
            }
            Msg::GetQueue => {
                self.api_agent.send(api::Request::GetQueue);
            }
            Msg::ApiResponse(response) => {
                if let api::Response::Success(api::ResponseData::Queue(queue)) = response {
                    self.queue = queue;
                    return true;
                }
            }
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
}
