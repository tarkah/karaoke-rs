use crate::pages::*;

use yew::prelude::*;
use yew_router::{prelude::*, switch::Permissive, Switch};

#[derive(Debug, Switch, Clone)]
pub enum AppRoute {
    #[to = "/!"]
    Index,
    #[to = "/songs"]
    Songs,
    #[to = "/artists"]
    Artists,
    #[to = "/artist/{artist_id}"]
    Artist(u64),
    #[to = "/queue"]
    Queue,
    #[to = "/page-not-found"]
    NotFound(Permissive<String>),
}

pub struct Model {}

impl Component for Model {
    type Message = ();
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        Model {}
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                { self.view_header() }
                <main class="container" role="main" style="padding-top: 100px">
                    { self.view_page() }
                </main>
            </div>
        }
    }
}

impl Model {
    fn view_header(&self) -> Html {
        html! {
            <div class="container fixed-top bg-white align-items-center border-bottom pt-2 pb-1">
                <div class="column">
                    <div class="row align-items-center">
                        <h2>{ "Karaoke-rs" }</h2>
                    </div>
                    <div class="row align-items-center">
                        <span>
                            <h5>
                                <a href="/">
                                <RouterAnchor<AppRoute> route=AppRoute::Index>{ "Home" }</RouterAnchor<AppRoute>>
                                </a>
                            </h5>
                        </span>
                        <span>
                            <h5>{ "\u{00a0}|\u{00a0}" }</h5>
                        </span>
                        <span>
                            <h5>
                                <a href="/songs">
                                <RouterAnchor<AppRoute> route=AppRoute::Songs>{ "Songs" }</RouterAnchor<AppRoute>>
                                </a>
                            </h5>
                        </span>
                        <span>
                            <h5>{ "\u{00a0}|\u{00a0}" }</h5>
                        </span>
                        <span>
                            <h5>
                                <a href="/artists">
                                <RouterAnchor<AppRoute> route=AppRoute::Artists>{ "Artists" }</RouterAnchor<AppRoute>>
                                </a>
                            </h5>
                        </span>
                        <span>
                            <h5>{ "\u{00a0}|\u{00a0}" }</h5>
                        </span>
                        <span>
                            <h5>
                                <a href="/queue">
                                <RouterAnchor<AppRoute> route=AppRoute::Queue>{ "Queue" }</RouterAnchor<AppRoute>>
                                </a>
                            </h5>
                        </span>
                    </div>
                </div>
            </div>
        }
    }

    fn view_page(&self) -> Html {
        html! {
            <Router<AppRoute, ()>
                render = Router::render(|switch: AppRoute| {
                    match switch {
                        AppRoute::Index => html!{<IndexPage />},
                        AppRoute::Songs => html!{<SongsPage />},
                        AppRoute::Artist(id) => html!{<ArtistPage artist_id=id />},
                        AppRoute::Artists => html!{<ArtistsPage />},
                        AppRoute::Queue => html!{<QueuePage />},
                        AppRoute::NotFound(Permissive(None)) => html!{"Page not found"},
                        AppRoute::NotFound(Permissive(Some(missed_route))) => html!{format!("Page '{}' not found", missed_route)},
                        _ => html!{"Page not found"},
                    }
                })
                redirect = Router::redirect(|route: Route| {
                    AppRoute::NotFound(Permissive(Some(route.route)))
                })
            />
        }
    }
}
