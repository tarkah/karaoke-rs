use crate::{
    agents::api,
    app::AppRoute,
    components::pagination::Pagination,
    model::{Artist, RequestParams, SortDirection, SortKey},
};
use log::trace;
use yew::prelude::*;
use yew_router::prelude::*;

pub enum Msg {
    GetArtists,
    TablePageUpdate(u32),
    SortUpdate(SortKey),
    Search(String),
    ApiResponse(api::Response),
}

pub struct ArtistsPage {
    link: ComponentLink<ArtistsPage>,
    api_agent: Box<dyn Bridge<api::ApiAgent>>,
    artists: Vec<Artist>,
    artists_fetched: bool,
    search: Option<String>,
    page_selection: Option<u32>,
    total_pages: Option<u32>,
    sort_key: Option<SortKey>,
    sort_direction: Option<SortDirection>,
}

impl Component for ArtistsPage {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let api_agent = api::ApiAgent::bridge(link.callback(Msg::ApiResponse));

        ArtistsPage {
            link,
            api_agent,
            artists: vec![],
            artists_fetched: false,
            search: None,
            page_selection: None,
            total_pages: None,
            sort_key: None,
            sort_direction: None,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        self.link.send_message(Msg::GetArtists);
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::GetArtists => {
                let params = RequestParams {
                    page: self.page_selection,
                    query: self.search_value(),
                    sort_key: self.sort_key,
                    sort_direction: self.sort_direction,
                    ..RequestParams::default()
                };
                self.api_agent.send(api::Request::GetArtists(params));
            }
            Msg::TablePageUpdate(n) => {
                self.page_selection = Some(n);
                self.update(Msg::GetArtists);
            }
            Msg::SortUpdate(key) => {
                let direction = match self.sort_direction {
                    None => SortDirection::Desc,
                    Some(SortDirection::Asc) => SortDirection::Desc,
                    Some(SortDirection::Desc) => SortDirection::Asc,
                };
                let direction =
                    if key != self.sort_key.unwrap_or(SortKey::Artist) && key == SortKey::Artist {
                        SortDirection::Asc
                    } else if key != self.sort_key.unwrap_or(SortKey::Artist)
                        && key == SortKey::NumSongs
                    {
                        SortDirection::Desc
                    } else {
                        direction
                    };

                self.sort_direction = Some(direction);
                self.sort_key = Some(key);

                self.update(Msg::GetArtists);
            }

            Msg::Search(value) => {
                trace!("Search Input: {}", value);
                self.search = Some(value);
                self.page_selection = None;
                self.update(Msg::GetArtists);
            }
            Msg::ApiResponse(response) => {
                if let api::Response::Success(api::ResponseData::Artists {
                    artists,
                    total_pages,
                }) = response
                {
                    self.artists = artists;
                    self.total_pages = Some(total_pages);
                    self.artists_fetched = true;
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

    fn sort_class(&self, sort_key: SortKey) -> &str {
        if let Some(key) = self.sort_key {
            if key == sort_key {
                match self.sort_direction {
                    None => "table__sortable-header",
                    Some(SortDirection::Asc) => "table__sortable-header--asc",
                    Some(SortDirection::Desc) => "table__sortable-header--desc",
                }
            } else {
                "table__sortable-header"
            }
        } else {
            "table__sortable-header"
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
                                    <th onclick=self.link.callback(|_| Msg::SortUpdate(SortKey::Artist))
                                        class=self.sort_class(SortKey::Artist)>{ "Artist" }</th>
                                    <th onclick=self.link.callback(|_| Msg::SortUpdate(SortKey::NumSongs))
                                        class=self.sort_class(SortKey::NumSongs)>{ "# Songs" }</th>
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
}
