use yew::prelude::*;

pub struct IndexPage {}

impl Component for IndexPage {
    type Message = ();
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        IndexPage {}
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html<Self> {
        html! {
            <div class="mt-2">
                <h3>{ "Welcome to this simple karaoke site!" }</h3>
                <p>{ "Add songs to the queue or play now to skip to the front." }</p>
            </div>
        }
    }
}
