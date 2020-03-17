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
    Favorite((bool, u64)),
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
            Msg::Favorite((favorite, id)) => {
                if favorite {
                    self.api_agent.send(api::Request::RemoveFavorite(id));
                } else {
                    self.api_agent.send(api::Request::AddFavorite(id));
                }
                self.update(Msg::GetQueue);
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
                    <button class="button button-queue-action"
                        role="button" aria-pressed="true" onclick=self.link.callback(|_| Msg::Clear)>{ "Clear Queue" }</button>
                    <button class="button button-queue-action"
                        role="button" aria-pressed="true" onclick=self.link.callback(|_| Msg::Next)>{ "Next Song" }</button>
                    <button class="button button-queue-action"
                        role="button" aria-pressed="true" onclick=self.link.callback(|_| Msg::Stop)>{ "Stop" }</button>
                </div>
                { self.view_table() }
            </div>
        }
    }
}

impl QueuePage {
    fn view_row(&self, idx: usize, song: Song) -> Html {
        let song_id = song.id;
        let favorite = song.favorite;

        html! {
            <tr>
                <th class="text-center">{ idx + 1 }</th>
                <td>{ song.name }</td>
                <td class="text-center">{ song.artist_name }</td>
                <td class="heart-center">
                    <button onclick=self.link.callback(move |_| Msg::Favorite((favorite, song_id))) class="button button-table"
                        role="button" aria-pressed="true">{ self.view_favorite(favorite) }</button>
                </td>
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
                                <th class="text-center">{ "#" }</th>
                                <th>{ "Song" }</th>
                                <th class="text-center">{ "Artist" }</th>
                                <th><div class="heart-header heart-center">{ "🤍" }</div></th>
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

    fn view_favorite(&self, favorite: bool) -> &str {
        if favorite {
            "♥️"
        } else {
            "🤍"
        }
    }
}
