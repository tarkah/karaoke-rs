use crate::{
    agents::api,
    app::AppRoute,
    components::pagination::Pagination,
    model::{RequestParams, Song},
};
use log::trace;
use yew::prelude::*;
use yew_router::prelude::*;

pub enum Msg {
    GetSongs,
    Add(u64),
    PlayNow(u64),
    TablePageUpdate(u32),
    Search(String),
    ApiResponse(api::Response),
}

pub struct SongsPage {
    link: ComponentLink<SongsPage>,
    api_agent: Box<dyn Bridge<api::ApiAgent>>,
    songs_fetched: bool,
    songs: Vec<Song>,
    search: Option<String>,
    page_selection: Option<u32>,
    total_pages: Option<u32>,
}

impl Component for SongsPage {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let api_agent = api::ApiAgent::bridge(link.callback(Msg::ApiResponse));

        SongsPage {
            link,
            api_agent,
            songs: vec![],
            songs_fetched: false,
            search: None,
            page_selection: None,
            total_pages: None,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.link.send_message(Msg::GetSongs);
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::GetSongs => {
                let params = RequestParams {
                    page: self.page_selection,
                    query: self.search_value(),
                    ..RequestParams::default()
                };
                self.api_agent.send(api::Request::GetSongs(params));
            }
            Msg::Add(id) => {
                self.api_agent.send(api::Request::AddSong(id));
            }
            Msg::PlayNow(id) => {
                self.api_agent.send(api::Request::PlaySong(id));
            }
            Msg::TablePageUpdate(n) => {
                self.page_selection = Some(n);
                self.update(Msg::GetSongs);
            }
            Msg::Search(value) => {
                trace!("Search Input: {}", value);
                self.search = Some(value);
                self.page_selection = None;
                self.update(Msg::GetSongs);
            }
            Msg::ApiResponse(response) => {
                if let api::Response::Success(api::ResponseData::Songs { songs, total_pages }) =
                    response
                {
                    self.songs = songs;
                    self.total_pages = Some(total_pages);
                    self.songs_fetched = true;
                    return true;
                }
            }
        }
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                { self.view_table() }
            </div>
        }
    }
}

impl SongsPage {
    fn total_pages(&self) -> u32 {
        self.total_pages.unwrap_or(0)
    }

    fn current_page(&self) -> u32 {
        self.page_selection.unwrap_or(1)
    }

    fn search_value(&self) -> Option<String> {
        if let Some(ref query) = self.search {
            if query == "" {
                None
            } else {
                Some(query.clone())
            }
        } else {
            None
        }
    }

    fn view_row(&self, song: Song) -> Html {
        let song_id = song.id;

        html! {
            <tr>
                <td>{ song.name }</td>
                <td>
                    <RouterAnchor<AppRoute> route=AppRoute::Artist(song.artist_id) classes="artist-link">{ song.artist_name }</ RouterAnchor<AppRoute>>
                </td>
                <td>
                    <button onclick=self.link.callback(move |_| Msg::Add(song_id)) class="button button-table"
                        role="button" aria-pressed="true">{ "Add" }</button>
                </td>
                <td>
                    <button onclick=self.link.callback(move |_| Msg::PlayNow(song_id)) class="button button-table"
                        role="button" aria-pressed="true">{ "Play" }</button>
                </td>
            </tr>
        }
    }
    fn view_table(&self) -> Html {
        if self.songs_fetched {
            html! {
                <div>
                    <div>
                        <input class="input" type="text" placeholder="Search"
                            oninput=self.link.callback(|input: InputData| Msg::Search(input.value))></input>
                    </div>
                    <div>
                        <table class="table">
                            <thead>
                                <tr>
                                    <th>{ "Song" }</th>
                                    <th>{ "Artist" }</th>
                                    <th></th>
                                    <th></th>
                                </tr>
                            </thead>
                            <tbody>
                                {
                                    for self.songs.iter().map(|song| {
                                        self.view_row(song.clone())
                                    })
                                }
                            </tbody>
                        </table>
                    </div>
                    <Pagination onupdate=self.link.callback(Msg::TablePageUpdate)
                                current_page={ self.current_page() }
                                total_pages={ self.total_pages() }
                    />
                </div>
            }
        } else {
            html! {}
        }
    }
}
