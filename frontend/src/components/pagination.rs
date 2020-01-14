use yew::prelude::*;

pub enum Msg {
    TablePageChange(u32),
    TablePageUp,
    TablePageDown,
}

#[derive(Properties, Clone)]
pub struct Props {
    #[props(required)]
    pub onupdate: Callback<u32>,
    pub total_pages: u32,
    pub current_page: u32,
}

pub struct Pagination {
    link: ComponentLink<Self>,
    onupdate: Callback<u32>,
    total_pages: u32,
    current_page: u32,
}

impl Component for Pagination {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Pagination {
            link,
            onupdate: props.onupdate,
            total_pages: props.total_pages,
            current_page: props.current_page,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.total_pages = props.total_pages;
        self.current_page = props.current_page;
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::TablePageChange(n) => self.onupdate.emit(n),
            Msg::TablePageUp => {
                if self.current_page < self.total_pages {
                    self.onupdate.emit(self.current_page + 1)
                }
            }
            Msg::TablePageDown => {
                if self.current_page > 1 {
                    self.onupdate.emit(self.current_page - 1)
                }
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="table-paging">
                <button class="table-paging__btn" onclick=self.link.callback(|_| Msg::TablePageDown)
                    disabled={ self.current_page==1 }>{ "«" }</button>
                { self.view_pagination_button_first() }
                { self.view_pagination_buttons_first() }
                { self.view_pagination_delimiter_first() }
                { self.view_pagination_button_middle() }
                { self.view_pagination_delimiter_last() }
                { self.view_pagination_buttons_last() }
                { self.view_pagination_button_last() }
                <button class="table-paging__btn" onclick=self.link.callback(|_| Msg::TablePageUp)
                    disabled={ self.current_page==self.total_pages }>{ "»" }</button>
            </div>
        }
    }
}

impl Pagination {
    fn view_pagination_button_is_active(&self, page: u32) -> &str {
        let current_page = self.current_page;

        if current_page == page {
            "table-paging__btn--active"
        } else {
            "table-paging__btn"
        }
    }

    fn view_pagination_button(&self, page: u32) -> Html {
        html! {
            <button class={ self.view_pagination_button_is_active(page) }
                onclick=self.link.callback(move |_| Msg::TablePageChange(page))>{ page }</button>
        }
    }

    fn view_pagination_button_first(&self) -> Html {
        self.view_pagination_button(1)
    }

    fn view_pagination_delimiter_first(&self) -> Html {
        let total_pages = self.total_pages;
        let current_page = self.current_page;

        if total_pages > 5 && current_page > 3 {
            html! {
                <button class="table-paging__btn">{ "..." }</button>
            }
        } else {
            html! {}
        }
    }

    fn view_pagination_delimiter_last(&self) -> Html {
        let total_pages = self.total_pages;
        let current_page = self.current_page;

        if total_pages > 5 && total_pages - current_page > 2 {
            html! {
                <button class="table-paging__btn">{ "..." }</button>
            }
        } else {
            html! {}
        }
    }

    fn view_pagination_button_last(&self) -> Html {
        let total_pages = self.total_pages;

        if total_pages > 5 {
            self.view_pagination_button(total_pages)
        } else {
            html! {}
        }
    }

    fn view_pagination_button_middle(&self) -> Html {
        let total_pages = self.total_pages;
        let current_page = self.current_page;
        let total_pages_safe = if total_pages < 2 { 2 } else { total_pages };

        if total_pages > 5 && current_page > 3 && current_page < total_pages_safe - 2 {
            self.view_pagination_button(current_page)
        } else {
            html! {}
        }
    }

    fn view_pagination_buttons_first(&self) -> Html {
        let total_pages = self.total_pages;
        let current_page = self.current_page;

        let max = if total_pages > 5 { 3 } else { 5 };
        let through = if total_pages > 5 { 4 } else { 6 };

        html! {
            { for (2..through).filter_map(|page| {
                if current_page <= max && total_pages >= page {
                    Some(self.view_pagination_button(page))
                } else {
                    None
                }
            }) }
        }
    }

    fn view_pagination_buttons_last(&self) -> Html {
        let total_pages = self.total_pages;
        let current_page = self.current_page;
        let total_pages_safe = if total_pages < 2 { 2 } else { total_pages };

        html! {
            {
                for (total_pages_safe-2..total_pages_safe).filter_map(|page| {
                    if total_pages > 5 && current_page > 3 && current_page >= total_pages_safe - 2 {
                        Some(self.view_pagination_button(page))
                    } else {
                        None
                    }
                })
            }
        }
    }
}
