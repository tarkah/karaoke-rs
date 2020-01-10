use crate::{
    components::pagination::Pagination,
    model::{ApiResponse, Artist, DataType, PostSong, RequestParams, Song},
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

pub enum Msg {
    FetchSongs,
    StoreSongData { songs: Vec<Song>, total_pages: u32 },
    FetchArtist,
    StoreArtistData(Artist),
    TablePageUpdate(u32),
    Search(String),
    Add(u64),
    PlayNow(u64),
    Noop,
}

#[derive(Properties, Clone)]
pub struct Props {
    #[props(required)]
    pub artist_id: u64,
}

pub struct ArtistPage {
    fetch_service: FetchService,
    fetch_task_1: Option<FetchTask>,
    fetch_task_2: Option<FetchTask>,
    link: ComponentLink<ArtistPage>,
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
        ArtistPage {
            fetch_service: FetchService::new(),
            fetch_task_1: None,
            fetch_task_2: None,
            link,
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
        self.link.send_message(Msg::FetchArtist);
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FetchSongs => {
                let fetch_task = self.fetch_songs();
                self.fetch_task_2 = Some(fetch_task);
            }
            Msg::StoreSongData { songs, total_pages } => {
                self.songs = songs;
                self.total_pages = Some(total_pages);
                self.songs_fetched = true;
                return true;
            }
            Msg::FetchArtist => {
                let fetch_task = self.fetch_artist();
                self.fetch_task_1 = Some(fetch_task);
                self.update(Msg::FetchSongs);
            }
            Msg::StoreArtistData(artist) => {
                self.artist_name = Some(artist.name);
                self.artist_fetched = true;
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
                self.fetch_task_2 = Some(fetch_task);
            }
            Msg::PlayNow(id) => {
                let fetch_task = self.post_song(id, "playnow");
                self.fetch_task_2 = Some(fetch_task);
            }
            Msg::Noop => {}
        }
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                {
                    if self.artist_fetched {
                        html! {
                            <div class="align-items-center mb-2">
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
                <td class="text-center">
                    <button onclick=self.link.callback(move |_| Msg::Add(song_id)) class="btn btn-secondary btn-sm active"
                        role="button" aria-pressed="true">{ "Add" }</button>
                </td>
                <td class="text-center">
                    <button onclick=self.link.callback(move |_| Msg::PlayNow(song_id)) class="btn btn-primary btn-sm active"
                        role="button" aria-pressed="true">{ "Play" }</button>
                </td>
            </tr>
        }
    }

    fn view_table(&self) -> Html {
        if self.songs_fetched {
            html! {
                <div>
                    <div style="width: 50%; margin-bottom: 16px;">
                        <input class="form-control" type="text" placeholder="Search"
                            oninput=self.link.callback(|input: InputData| Msg::Search(input.value))></input>
                    </div>
                    <div class="justify-content-center">
                        <table class="table table-striped table-bordered">
                            <thead>
                                <tr>
                                    <th scope="col">{ "Song" }</th>
                                    <th scope="col"></th>
                                    <th scope="col"></th>
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
            artist_id: Some(self.artist_id),
        };
        let params: String = serde_urlencoded::to_string(&params).unwrap();

        let request = Request::get(&format!("/api/songs?{}", params))
            .body(Nothing)
            .unwrap();
        self.fetch_service.fetch(request, callback)
    }

    fn fetch_artist(&mut self) -> FetchTask {
        trace!("Fetching artist from API");

        let callback = self.link.callback(
            move |response: Response<Json<Result<ApiResponse, Error>>>| {
                let Json(body) = response.into_body();

                if let Ok(ApiResponse::SuccessGet(data)) = body {
                    if let DataType::Artists(artists) = data.data {
                        if artists.len() == 1 {
                            Msg::StoreArtistData(artists[0].clone())
                        } else {
                            trace!("No artist found");
                            Msg::Noop
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
            artist_id: Some(self.artist_id),
        };
        let params: String = serde_urlencoded::to_string(&params).unwrap();

        let request = Request::get(&format!("/api/artists?{}", params))
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
