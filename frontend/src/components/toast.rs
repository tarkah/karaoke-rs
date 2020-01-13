use serde::{Deserialize, Serialize};
use std::time::Duration;
use yew::{
    prelude::*,
    services::timeout::{TimeoutService, TimeoutTask},
};

pub enum Msg {
    RemoveShow,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ToastStatus {
    Success,
    Error,
}

impl std::fmt::Display for ToastStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ToastStatus::Success => "success",
            ToastStatus::Error => "error",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ToastBody {
    pub status: ToastStatus,
    pub title: String,
    pub delay: u64,
}

#[derive(Clone, Properties)]
pub struct Props {
    #[props(required)]
    pub body: ToastBody,
}

pub struct Toast {
    body: ToastBody,
    #[allow(dead_code)]
    timeout_task: Option<TimeoutTask>,
    toast_show: bool,
}

impl Component for Toast {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Toast {
        let mut timeout_service = TimeoutService::new();
        let callback = link.callback(|_| Msg::RemoveShow);
        let timeout_task = timeout_service.spawn(Duration::from_millis(500), callback);

        Toast {
            body: props.body,
            timeout_task: Some(timeout_task),
            toast_show: true,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::RemoveShow => {
                self.toast_show = false;
            }
        }
        true
    }

    fn view(&self) -> Html {
        let toast_class = format!("toast toast-{} {}", self.body.status, self.toast_show());

        html! {
            <div class=toast_class>
                <div class="toast-header">
                    { self.body.title.clone() }
                </div>
            </div>
        }
    }
}

impl Toast {
    fn toast_show(&self) -> &str {
        if self.toast_show {
            "toast-show"
        } else {
            ""
        }
    }
}
