use crate::components::toast::ToastBody;
use serde::{Deserialize, Serialize};
use yew::worker::*;

#[derive(Serialize, Deserialize)]
pub enum Msg {
    NewToast(ToastBody),
}

pub struct ToastAgent {
    link: AgentLink<ToastAgent>,
    bridged_components: Vec<HandlerId>,
}

impl Agent for ToastAgent {
    type Reach = Context;
    type Message = Msg;
    type Input = Msg;
    type Output = Msg;

    fn create(link: AgentLink<Self>) -> Self {
        ToastAgent {
            link,
            bridged_components: vec![],
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.bridged_components.push(id);
    }

    fn disconnected(&mut self, id: HandlerId) {
        let idx = self
            .bridged_components
            .iter()
            .position(|bridged_id| *bridged_id == id);
        if let Some(idx) = idx {
            self.bridged_components.remove(idx);
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::NewToast(toast) => {
                for id in self.bridged_components.iter() {
                    self.link.respond(*id, Msg::NewToast(toast.clone()));
                }
            }
        }
    }

    fn handle_input(&mut self, msg: Self::Input, _: HandlerId) {
        match msg {
            Msg::NewToast(toast) => {
                self.link.callback(Msg::NewToast).emit(toast);
            }
        }
    }
}
