use crate::agents::player;
use anyhow::{format_err, Error};
use js_sys::JsString;
use log::{error, trace};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlDivElement, ImageData, Window};
use yew::{
    prelude::*,
    services::{
        resize::{ResizeTask, WindowDimensions},
        ResizeService,
    },
};

pub enum Msg {
    Resize(WindowDimensions),
    Player(player::Response),
    UserInputReceived,
    Error(Error),
}

#[derive(Properties, Clone)]
pub struct Props {
    #[props(required)]
    pub port_ws: u16,
    #[props(required)]
    pub fullscreen: bool,
    #[props(required)]
    pub scale: f32,
    #[props(required)]
    pub disable_background: bool,
}

pub struct PlayerPage {
    link: ComponentLink<Self>,
    #[allow(dead_code)]
    player_agent: Box<dyn Bridge<player::PlayerAgent>>,
    port_ws: u16,
    fullscreen: bool,
    scale: f32,
    disable_background: bool,
    window: Window,
    player_canvas: Option<HtmlCanvasElement>,
    hidden_canvas: Option<HtmlCanvasElement>,
    player_render_context: Option<CanvasRenderingContext2d>,
    hidden_render_context: Option<CanvasRenderingContext2d>,
    #[allow(dead_code)]
    resize_task: ResizeTask,
    width: u32,
    height: u32,
}

impl Component for PlayerPage {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut resize_service = ResizeService::new();
        let callback = link.callback(Msg::Resize);
        let resize_task = resize_service.register(callback);

        let callback = link.callback(Msg::Player);
        let player_agent = player::PlayerAgent::bridge(callback);

        let window = get_window();

        PlayerPage {
            link,
            window,
            player_agent,
            port_ws: props.port_ws,
            fullscreen: props.fullscreen,
            scale: props.scale,
            disable_background: props.disable_background,
            player_canvas: None,
            hidden_canvas: None,
            player_render_context: None,
            hidden_render_context: None,
            resize_task,
            width: 0,
            height: 0,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Resize(dimensions) => {
                self.resize(dimensions);
                return true;
            }
            Msg::Player(response) => match response {
                player::Response::RenderFrame {
                    mut cdg_frame,
                    background,
                } => {
                    let image_data = ImageData::new_with_u8_clamped_array_and_sh(
                        wasm_bindgen::Clamped(cdg_frame.as_mut_slice()),
                        300,
                        216,
                    )
                    .unwrap();

                    self.render_frame(image_data, background);
                }
                player::Response::ClearCanvas => {
                    self.clear_canvas();
                }
                player::Response::UserInputNeeded => {
                    self.show_modal();
                }
            },
            Msg::UserInputReceived => {
                self.hide_modal();

                self.player_agent.send(player::Request::UserInputReceived);
            }
            Msg::Error(e) => {
                error!("ERROR: {}", e);
            }
        }
        false
    }

    fn mounted(&mut self) -> ShouldRender {
        self.player_agent.send(player::Request::Port(self.port_ws));

        if let Err(e) = self.on_mounted() {
            self.link.send_message(Msg::Error(e));
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <>
                <canvas id="player"/>
                <canvas id="hidden" width="300" height="216" />
                <img id="player-background" src="player_background.png" />
                <div id="input-modal" class="modal">
                    <div class="modal-content">
                        <p>{ "Get ready for some karaoke!" }</p>
                        <button onclick=self.link.callback(|_| Msg::UserInputReceived)
                                class="modal-close">{ "Load Player" }</button>
                    </div>
                </div>
            </>
        }
    }
}

impl PlayerPage {
    fn on_mounted(&mut self) -> Result<(), Error> {
        self.load_canvas()?;
        self.load_render_context()?;

        let dimensions = self.dimensions()?;
        self.resize(dimensions);

        Ok(())
    }

    fn load_canvas(&mut self) -> Result<(), Error> {
        let player_canvas = get_canvas(&self.window, "player")
            .ok_or_else(|| format_err!("Failed to get player canvas"))?;
        let hidden_canvas = get_canvas(&self.window, "hidden")
            .ok_or_else(|| format_err!("Failed to get hidden canvas"))?;
        self.player_canvas = Some(player_canvas);
        self.hidden_canvas = Some(hidden_canvas);

        Ok(())
    }

    fn load_render_context(&mut self) -> Result<(), Error> {
        let player_render_context = self
            .player_canvas
            .as_ref()
            .ok_or_else(|| format_err!("Failed to get canvas"))?
            .get_context("2d")
            .map_err(|_| format_err!("Failed to get 2d rendering context"))?
            .ok_or_else(|| format_err!("Failed to get 2d rendering context"))?
            .unchecked_into();
        self.player_render_context = Some(player_render_context);

        let hidden_render_context = self
            .hidden_canvas
            .as_ref()
            .ok_or_else(|| format_err!("Failed to get canvas"))?
            .get_context("2d")
            .map_err(|_| format_err!("Failed to get 2d rendering context"))?
            .ok_or_else(|| format_err!("Failed to get 2d rendering context"))?
            .unchecked_into();
        self.hidden_render_context = Some(hidden_render_context);
        Ok(())
    }

    fn dimensions(&mut self) -> Result<WindowDimensions, Error> {
        let width = self
            .window
            .inner_width()
            .map_err(|_| format_err!("Failed to get width on resize"))?
            .as_f64()
            .ok_or_else(|| format_err!("Conversion failed"))?;
        let height = self
            .window
            .inner_height()
            .map_err(|_| format_err!("Failed to get height on resize"))?
            .as_f64()
            .ok_or_else(|| format_err!("Conversion failed"))?;
        Ok(WindowDimensions {
            width: width as i32,
            height: height as i32,
        })
    }

    fn resize(&mut self, dimensions: WindowDimensions) {
        self.width = dimensions.width as u32;
        self.height = dimensions.height as u32;

        self.player_canvas.as_mut().unwrap().set_width(self.width);
        self.player_canvas.as_mut().unwrap().set_height(self.height);
        trace!("Canvas resized to: {}x{}", self.width, self.height);
    }

    fn render_frame(&mut self, image_data: ImageData, background: (f32, f32, f32, f32)) {
        let x = if self.fullscreen {
            0.0
        } else {
            self.width as f64 / 2.0 - (300.0 * self.scale as f64) / 2.0
        };
        let y = if self.fullscreen {
            0.0
        } else {
            self.height as f64 / 2.0 - (216.0 * self.scale as f64) / 2.0
        };
        let width = if self.fullscreen {
            self.width as f64
        } else {
            300.0 * self.scale as f64
        };
        let height = if self.fullscreen {
            self.height as f64
        } else {
            216.0 * self.scale as f64
        };

        let player_render_context = self.player_render_context.as_ref().unwrap();
        let hidden_render_context = self.hidden_render_context.as_ref().unwrap();
        let hidden_canvas = self.hidden_canvas.as_ref().unwrap();

        let _ = hidden_render_context.put_image_data(&image_data, 0.0, 0.0);

        if !self.fullscreen {
            let color = if self.disable_background {
                format!("rgba({}, {}, {}, {})", 0, 0, 0, 1)
            } else {
                format!(
                    "rgba({}, {}, {}, {})",
                    background.0, background.1, background.2, background.3
                )
            };

            let color: JsString = color.as_str().into();
            player_render_context.set_fill_style(&color);
            player_render_context.fill_rect(0.0, 0.0, self.width as f64, self.height as f64);
        }

        let color: JsString = format!("rgba({}, {}, {}, {})", 0, 0, 0, 1).as_str().into();
        player_render_context.set_fill_style(&color);
        player_render_context.fill_rect(x, y, width, height);

        let _ = player_render_context.draw_image_with_html_canvas_element_and_dw_and_dh(
            &hidden_canvas,
            x,
            y,
            width,
            height,
        );
    }

    fn clear_canvas(&mut self) {
        let player_render_context = self.player_render_context.as_ref().unwrap();
        player_render_context.clear_rect(0.0, 0.0, self.width as f64, self.height as f64);
    }

    fn show_modal(&mut self) {
        if let Some(modal) = get_modal(&self.window, "input-modal") {
            let style = &modal.style();
            let _ = style.set_property("display", "block");
        }

        trace!("Modal shown");
    }

    fn hide_modal(&mut self) {
        if let Some(modal) = get_modal(&self.window, "input-modal") {
            let style = &modal.style();
            let _ = style.set_property("display", "none");
        }

        trace!("Modal closed");
    }
}

fn get_window() -> Window {
    web_sys::window().unwrap()
}

fn get_canvas(window: &Window, id: &str) -> Option<HtmlCanvasElement> {
    Some(window.document()?.get_element_by_id(id)?.unchecked_into())
}

fn get_modal(window: &Window, id: &str) -> Option<HtmlDivElement> {
    Some(window.document()?.get_element_by_id(id)?.unchecked_into())
}
