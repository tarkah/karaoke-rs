use failure::Error;
use serde::{Deserialize, Serialize};
use yew::format::Json;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew::worker::*;

#[derive(Serialize, Deserialize)]
pub struct WsMessage {
    pub command: String,
}

pub enum Msg {
    WsReceived(Json<Result<WsMessage, Error>>),
    WsStatus(WebSocketStatus),
}

#[derive(Serialize, Deserialize)]
pub enum Request {
    WsSend(WsMessage),
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    RenderFrame(Vec<u8>),
    ClearCanvas,
}

pub struct PlayerAgent {
    #[allow(dead_code)]
    link: AgentLink<PlayerAgent>,
    bridged_component: Option<HandlerId>,
    ws_task: Option<WebSocketTask>,
}

impl Agent for PlayerAgent {
    type Reach = Job;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        let mut ws_service = WebSocketService::new();
        let callback = link.callback(Msg::WsReceived);
        let notification = link.callback(Msg::WsStatus);

        let ws_task = ws_service
            .connect(&get_ws_host(), callback, notification)
            .ok();

        PlayerAgent {
            link,
            bridged_component: None,
            ws_task,
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.bridged_component = Some(id);
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::WsReceived(Json(response)) => match response {
                Ok(data) => {
                    log::trace!("Websocket Received command: {}", data.command);
                }
                Err(e) => {
                    log::trace!("Websocket Error: {}", e);
                }
            },
            Msg::WsStatus(status) => match status {
                WebSocketStatus::Error => log::trace!("Websocket failure"),
                WebSocketStatus::Closed => log::trace!("Websocket closed"),
                WebSocketStatus::Opened => log::trace!("Websocket connection established"),
            },
        }
    }

    fn handle_input(&mut self, msg: Self::Input, _: HandlerId) {
        match msg {
            Request::WsSend(data) => {
                self.ws_task.as_mut().unwrap().send(Json(&data));
            }
        }
    }
}

fn get_ws_host() -> String {
    let window = web_sys::window().unwrap();
    let location = window.location();

    let protocol = location.protocol().unwrap();
    let hostname = location.hostname().unwrap();

    let host = match protocol.as_str() {
        "https:" => format!("wss://{}:9000", hostname),
        _ => format!("ws://{}:9000", hostname),
    };

    host
}
