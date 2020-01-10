use yew::prelude::*;

pub enum Msg {
    TablePageChange(u32),
    TablePageUp,
    TablePageDown,
}

#[derive(Properties)]
pub struct Props {
    #[props(required)]
    pub onupdate: Callback<u32>,
    pub total_pages: u32,
    pub current_page: u32,
}

pub struct Pagination {
    onupdate: Callback<u32>,
    total_pages: u32,
    current_page: u32,
}

impl Component for Pagination {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Pagination {
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

    fn view(&self) -> Html<Self> {
        let total_pages = self.total_pages;
        let current_page = self.current_page;

        html! {
            <ul class="pagination">
                <li class={ if current_page==1 {"page-item disabled"} else {"page-item"} }>
                    <a class="page-link" href="#" onclick=|_| Msg::TablePageDown>{ "«" }</a>
                </li>
                { self.view_pagination_button_first() }
                { self.view_pagination_buttons_first() }
                { self.view_pagination_delimiter_first() }
                { self.view_pagination_button_middle() }
                { self.view_pagination_delimiter_last() }
                { self.view_pagination_buttons_last() }
                { self.view_pagination_button_last() }
                <li class={ if current_page==total_pages || total_pages==0 {"page-item disabled"} else {"page-item"} }>
                    <a class="page-link" href="#" onclick=|_| Msg::TablePageUp>{ "»" }</a>
                </li>
            </ul>
        }
    }
}

impl Pagination {
    fn view_pagination_button_is_active(&self, page: u32) -> &str {
        let current_page = self.current_page;

        if current_page == page {
            "page-item active"
        } else {
            "page-item"
        }
    }

    fn view_pagination_button(&self, page: u32) -> Html<Self> {
        html! {
            <li class={ self.view_pagination_button_is_active(page) }>
                <a class="page-link" href="#" onclick=|_| Msg::TablePageChange(page)>{ page }</a>
            </li>
        }
    }

    fn view_pagination_button_first(&self) -> Html<Self> {
        self.view_pagination_button(1)
    }

    fn view_pagination_delimiter_first(&self) -> Html<Self> {
        let total_pages = self.total_pages;
        let current_page = self.current_page;

        if total_pages > 5 && current_page > 3 {
            html! {
                <li class="page-item disabled"><a class="page-link" href="#">{ "..." }</a></li>
            }
        } else {
            html! {}
        }
    }

    fn view_pagination_delimiter_last(&self) -> Html<Self> {
        let total_pages = self.total_pages;
        let current_page = self.current_page;

        if total_pages > 5 && total_pages - current_page > 2 {
            html! {
                <li class="page-item disabled"><a class="page-link" href="#">{ "..." }</a></li>
            }
        } else {
            html! {}
        }
    }

    fn view_pagination_button_last(&self) -> Html<Self> {
        let total_pages = self.total_pages;

        if total_pages > 5 {
            self.view_pagination_button(total_pages)
        } else {
            html! {}
        }
    }

    fn view_pagination_button_middle(&self) -> Html<Self> {
        let total_pages = self.total_pages;
        let current_page = self.current_page;
        let total_pages_safe = if total_pages < 2 { 2 } else { total_pages };

        if total_pages > 5 && current_page > 3 && current_page < total_pages_safe - 2 {
            self.view_pagination_button(current_page)
        } else {
            html! {}
        }
    }

    fn view_pagination_buttons_first(&self) -> Html<Self> {
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

    fn view_pagination_buttons_last(&self) -> Html<Self> {
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
