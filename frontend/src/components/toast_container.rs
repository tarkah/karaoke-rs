use super::toast::{Toast, ToastBody};
use crate::agents::toast::{Msg as ToastAgentMsg, ToastAgent};
use std::time::Duration;
use yew::{
    prelude::*,
    services::timeout::{TimeoutService, TimeoutTask},
};

pub enum Msg {
    Agent(ToastAgentMsg),
    RemoveToast,
}

pub struct ToastContainer {
    link: ComponentLink<Self>,
    #[allow(dead_code)]
    toast_agent: Box<dyn Bridge<ToastAgent>>,
    timeout_service: TimeoutService,
    timeout_tasks: Vec<TimeoutTask>,
    toasts: Vec<ToastBody>,
}

impl Component for ToastContainer {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> ToastContainer {
        let timeout_service = TimeoutService::new();
        let toast_agent = ToastAgent::bridge(link.callback(Msg::Agent));

        ToastContainer {
            link,
            toast_agent,
            timeout_service,
            timeout_tasks: vec![],
            toasts: vec![],
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Agent(ToastAgentMsg::NewToast(toast)) => {
                let duration = Duration::from_millis(toast.delay);
                let callback = self.link.callback(|_| Msg::RemoveToast);
                let timeout_task = self.timeout_service.spawn(duration, callback);

                self.toasts.push(toast);
                self.timeout_tasks.push(timeout_task);
            }
            Msg::RemoveToast => {
                if !self.timeout_tasks.is_empty() {
                    let _ = self.timeout_tasks.remove(0);
                }

                if !self.toasts.is_empty() {
                    self.toasts.remove(0);
                } else {
                    return false;
                };
            }
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <div id="toast-container">
            {
                for self.toasts.iter().map(|toast| {
                    self.view_toast(toast)
                })
            }
            </div>
        }
    }
}

impl ToastContainer {
    fn view_toast(&self, toast: &ToastBody) -> Html {
        html! {
            <Toast body=toast/>
        }
    }
}
