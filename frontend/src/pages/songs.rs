use crate::{
    app::AppRoute,
    components::pagination::Pagination,
    model::{ApiResponse, DataType, PostSong, RequestParams, Song},
};
use failure::{format_err, Error};
use log::trace;
use yew::{
    format::{Json, Nothing},
    prelude::*,
    services::{
        fetch::{FetchTask, Request, Response},
        FetchService,
    },
};
use yew_router::prelude::*;

pub enum Msg {
    FetchSongs,
    StoreSongData { songs: Vec<Song>, total_pages: u32 },
    TablePageUpdate(u32),
    Search(String),
    Add(u64),
    PlayNow(u64),
    Noop,
}

pub struct SongsPage {
    fetch_service: FetchService,
    fetch_task: Option<FetchTask>,
    link: ComponentLink<SongsPage>,
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
        SongsPage {
            fetch_service: FetchService::new(),
            fetch_task: None,
            link,
            songs: vec![],
            songs_fetched: false,
            search: None,
            page_selection: None,
            total_pages: None,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.link.send_message(Msg::FetchSongs);
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FetchSongs => {
                let fetch_task = self.fetch_songs();
                self.fetch_task = Some(fetch_task);
            }
            Msg::StoreSongData { songs, total_pages } => {
                self.songs = songs;
                self.total_pages = Some(total_pages);
                self.songs_fetched = true;
                return true;
            }
            Msg::TablePageUpdate(n) => {
                self.page_selection = Some(n);
                self.update(Msg::FetchSongs);
            }
            Msg::Search(value) => {
                trace!("Search Input: {}", value);
                self.search = Some(value);
                self.page_selection = None;
                self.update(Msg::FetchSongs);
            }
            Msg::Add(id) => {
                let fetch_task = self.post_song(id, "add");
                self.fetch_task = Some(fetch_task);
            }
            Msg::PlayNow(id) => {
                let fetch_task = self.post_song(id, "playnow");
                self.fetch_task = Some(fetch_task);
            }
            Msg::Noop => {}
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
                    <button onclick=self.link.callback(move |_| Msg::Add(song_id)) class="button"
                        role="button" aria-pressed="true">{ "Add" }</button>
                </td>
                <td>
                    <button onclick=self.link.callback(move |_| Msg::PlayNow(song_id)) class="button"
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
                                    <th width=250></th>
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

    fn fetch_songs(&mut self) -> FetchTask {
        trace!("Fetching songs from API");

        let callback = self.link.callback(
            move |response: Response<Json<Result<ApiResponse, Error>>>| {
                let Json(body) = response.into_body();

                if let Ok(ApiResponse::SuccessGet(data)) = body {
                    if let DataType::Songs(songs) = data.data {
                        Msg::StoreSongData {
                            songs,
                            total_pages: data.total_pages.unwrap_or(0),
                        }
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

        let params = RequestParams {
            page: self.page_selection,
            query: self.search_value(),
            ..RequestParams::default()
        };
        let params: String = serde_urlencoded::to_string(&params).unwrap();

        let request = Request::get(&format!("/api/songs?{}", params))
            .body(Nothing)
            .unwrap();
        self.fetch_service.fetch(request, callback)
    }

    fn post_song(&mut self, id: u64, command: &str) -> FetchTask {
        let callback = self.link.callback(
            move |response: Response<Json<Result<ApiResponse, Error>>>| {
                let Json(body) = response.into_body();

                if let Ok(ApiResponse::SuccessPost(data)) = body {
                    if data.status == "ok" {
                        trace!("Song successfully added");
                        Msg::Noop
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

        let body = serde_urlencoded::to_string(PostSong { hash: id })
            .map_err(|_| format_err!("Failed to serialize data"));

        let request = Request::post(&format!("/api/{}", command))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .unwrap();
        self.fetch_service.fetch(request, callback)
    }
}