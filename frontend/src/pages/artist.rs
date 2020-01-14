use crate::{
    agents::api,
    components::pagination::Pagination,
    model::{RequestParams, Song},
};
use log::trace;
use yew::prelude::*;

pub enum Msg {
    GetSongs,
    GetArtist,
    Add(u64),
    PlayNow(u64),
    TablePageUpdate(u32),
    Search(String),
    ApiResponse(api::Response),
}

#[derive(Properties, Clone)]
pub struct Props {
    #[props(required)]
    pub artist_id: u64,
}

pub struct ArtistPage {
    link: ComponentLink<ArtistPage>,
    api_agent: Box<dyn Bridge<api::ApiAgent>>,
    artist_id: u64,
    artist_name: Option<String>,
    artist_fetched: bool,
    songs_fetched: bool,
    songs: Vec<Song>,
    search: Option<String>,
    page_selection: Option<u32>,
    total_pages: Option<u32>,
}

impl Component for ArtistPage {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let api_agent = api::ApiAgent::bridge(link.callback(Msg::ApiResponse));

        ArtistPage {
            link,
            api_agent,
            artist_id: props.artist_id,
            artist_name: None,
            songs: vec![],
            artist_fetched: false,
            songs_fetched: false,
            search: None,
            page_selection: None,
            total_pages: None,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.link.send_message(Msg::GetArtist);
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::GetSongs => {
                let params = RequestParams {
                    page: self.page_selection,
                    query: self.search_value(),
                    artist_id: Some(self.artist_id),
                };
                self.api_agent.send(api::Request::GetSongs(params));
            }
            Msg::GetArtist => {
                let params = RequestParams {
                    artist_id: Some(self.artist_id),
                    ..RequestParams::default()
                };
                self.api_agent.send(api::Request::GetArtists(params));
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
            Msg::ApiResponse(response) => match response {
                api::Response::Success(api::ResponseData::Songs { songs, total_pages }) => {
                    self.songs = songs;
                    self.total_pages = Some(total_pages);
                    self.songs_fetched = true;
                    return true;
                }
                api::Response::Success(api::ResponseData::Artists { artists, .. }) => {
                    self.artist_name = Some(artists[0].name.clone());
                    self.artist_fetched = true;
                    self.update(Msg::GetSongs);
                    return true;
                }
                _ => {}
            },
        }
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                {
                    if self.artist_fetched {
                        html! {
                            <div class="align-items-center">
                                <h1>{ self.artist_name.clone().unwrap() }</h1>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
                { self.view_table() }
            </div>
        }
    }
}

impl ArtistPage {
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
                    <button onclick=self.link.callback(move |_| Msg::Add(song_id)) class="button button-table"
                        role="button" aria-pressed="true">{ "Add" }</button>
                </td>
                <td>
                    <button onclick=self.link.callback(move |_| Msg::PlayNow(song_id)) class="button button-table"
                    role="button"  aria-pressed="true">{ "Play" }</button>
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
