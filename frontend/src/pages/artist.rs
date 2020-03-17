use crate::{
    agents::api,
    components::pagination::Pagination,
    model::{RequestParams, Song, SortDirection, SortKey},
};
use log::trace;
use yew::prelude::*;

pub enum Msg {
    GetSongs,
    GetArtist,
    Add(u64),
    PlayNow(u64),
    TablePageUpdate(u32),
    SortUpdate(SortKey),
    Search(String),
    Favorite((bool, u64)),
    ApiResponse(api::Response),
}

#[derive(Properties, Clone)]
pub struct Props {
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
    sort_key: Option<SortKey>,
    sort_direction: Option<SortDirection>,
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
            sort_key: None,
            sort_direction: None,
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
                    sort_key: self.sort_key,
                    sort_direction: self.sort_direction,
                    ..RequestParams::default()
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
            Msg::SortUpdate(key) => {
                let direction = match self.sort_direction {
                    None => SortDirection::Desc,
                    Some(SortDirection::Asc) => SortDirection::Desc,
                    Some(SortDirection::Desc) => SortDirection::Asc,
                };
                let direction = if key != self.sort_key.unwrap_or(SortKey::Song) {
                    SortDirection::Asc
                } else {
                    direction
                };

                self.sort_direction = Some(direction);
                self.sort_key = Some(key);

                self.update(Msg::GetSongs);
            }
            Msg::Search(value) => {
                trace!("Search Input: {}", value);
                self.search = Some(value);
                self.page_selection = None;
                self.update(Msg::GetSongs);
            }
            Msg::Favorite((favorite, id)) => {
                if favorite {
                    self.api_agent.send(api::Request::RemoveFavorite(id));
                } else {
                    self.api_agent.send(api::Request::AddFavorite(id));
                }
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

    fn view_row(&self, song: Song) -> Html {
        let song_id = song.id;
        let favorite = song.favorite;

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
                <td class="heart-center">
                    <button onclick=self.link.callback(move |_| Msg::Favorite((favorite, song_id))) class="button button-table"
                        role="button" aria-pressed="true">{ self.view_favorite(favorite) }</button>
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
                                    <th onclick=self.link.callback(|_| Msg::SortUpdate(SortKey::Song))
                                        class=self.sort_class(SortKey::Song)>{ "Song" }</th>
                                    <th></th>
                                    <th></th>
                                    <th><div class="heart-header heart-center">{ "🤍" }</div></th>
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

    fn view_favorite(&self, favorite: bool) -> &str {
        if favorite {
            "♥️"
        } else {
            "🤍"
        }
    }
}
