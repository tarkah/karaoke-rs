use crate::pages::*;

use yew::prelude::*;
use yew_router::{prelude::*, Switch};

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
    NotFound(Option<String>),
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

    fn view(&self) -> Html<Self> {
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
    fn view_header(&self) -> Html<Self> {
        html! {
            <div class="container fixed-top bg-white align-items-center border-bottom pt-2 pb-1">
                <div class="column">
                    <div class="row align-items-center">
                        <h2>{ "Karaoke-rs" }</h2>
                    </div>
                    <div class="row align-items-center">
                        <span>
                            <h5>
                                <RouterLink text=String::from("Home"), link="/", />
                            </h5>
                        </span>
                        <span>
                            <h5>{ "\u{00a0}|\u{00a0}" }</h5>
                        </span>
                        <span>
                            <h5>
                                <RouterLink text=String::from("Songs"), link="/songs", />
                            </h5>
                        </span>
                        <span>
                            <h5>{ "\u{00a0}|\u{00a0}" }</h5>
                        </span>
                        <span>
                            <h5>
                                <RouterLink text=String::from("Artists"), link="/artists", />
                            </h5>
                        </span>
                        <span>
                            <h5>{ "\u{00a0}|\u{00a0}" }</h5>
                        </span>
                        <span>
                            <h5>
                                <RouterLink text=String::from("Queue"), link="/queue", />
                            </h5>
                        </span>
                    </div>
                </div>
            </div>
        }
    }

    fn view_page(&self) -> Html<Self> {
        html! {
            <Router<AppRoute, ()>
                render = Router::render(|switch: AppRoute| {
                    match switch {
                        AppRoute::Index => html!{<IndexPage />},
                        AppRoute::Songs => html!{<SongsPage />},
                        AppRoute::Artist(id) => html!{<ArtistPage artist_id=id />},
                        AppRoute::Artists => html!{<ArtistsPage />},
                        AppRoute::Queue => html!{<QueuePage />},
                        AppRoute::NotFound(None) => html!{"Page not found"},
                        AppRoute::NotFound(Some(missed_route)) => html!{format!("Page '{}' not found", missed_route)},
                        _ => html!{"Page not found"},
                    }
                })
                redirect = Router::redirect(|route: Route| {
                    AppRoute::NotFound(Some(route.route))
                })
            />
        }
    }
}
