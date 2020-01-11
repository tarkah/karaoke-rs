use crate::{
    app::AppRoute,
    components::pagination::Pagination,
    model::{ApiResponse, Artist, DataType, RequestParams},
};
use failure::Error;
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
    FetchArtists,
    StoreArtistData {
        artists: Vec<Artist>,
        total_pages: u32,
    },
    TablePageUpdate(u32),
    Search(String),
    Noop,
}

pub struct ArtistsPage {
    fetch_service: FetchService,
    fetch_task: Option<FetchTask>,
    link: ComponentLink<ArtistsPage>,
    artists: Vec<Artist>,
    artists_fetched: bool,
    search: Option<String>,
    page_selection: Option<u32>,
    total_pages: Option<u32>,
}

impl Component for ArtistsPage {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        ArtistsPage {
            fetch_service: FetchService::new(),
            fetch_task: None,
            link,
            artists: vec![],
            artists_fetched: false,
            search: None,
            page_selection: None,
            total_pages: None,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.link.send_message(Msg::FetchArtists);
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FetchArtists => {
                let fetch_task = self.fetch_artists();
                self.fetch_task = Some(fetch_task);
            }
            Msg::StoreArtistData {
                artists,
                total_pages,
            } => {
                self.artists = artists;
                self.total_pages = Some(total_pages);
                self.artists_fetched = true;
                return true;
            }
            Msg::TablePageUpdate(n) => {
                self.page_selection = Some(n);
                self.update(Msg::FetchArtists);
            }
            Msg::Search(value) => {
                trace!("Search Input: {}", value);
                self.search = Some(value);
                self.page_selection = None;
                self.update(Msg::FetchArtists);
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

impl ArtistsPage {
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
    fn view_row(&self, artist: Artist) -> Html {
        html! {
            <tr>
                <td>
                    <RouterAnchor<AppRoute> route=AppRoute::Artist(artist.id) classes="artist-link">{ artist.name }</RouterAnchor<AppRoute>>
                </td>
                <td>{ artist.num_songs }</td>
            </tr>
        }
    }

    fn view_table(&self) -> Html {
        if self.artists_fetched {
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
                                    <th>{ "Artist" }</th>
                                    <th>{ "# Songs" }</th>
                                </tr>
                            </thead>
                            <tbody>
                                {
                                    for self.artists.iter().map(|artist| {
                                        self.view_row(artist.clone())
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

    fn fetch_artists(&mut self) -> FetchTask {
        trace!("Fetching songs from API");

        let callback = self.link.callback(
            move |response: Response<Json<Result<ApiResponse, Error>>>| {
                let Json(body) = response.into_body();

                if let Ok(ApiResponse::SuccessGet(data)) = body {
                    if let DataType::Artists(artists) = data.data {
                        Msg::StoreArtistData {
                            artists,
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

        let request = Request::get(&format!("/api/artists?{}", params))
            .body(Nothing)
            .unwrap();
        self.fetch_service.fetch(request, callback)
    }
}