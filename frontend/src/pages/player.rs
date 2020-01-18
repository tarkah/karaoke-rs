use crate::agents::player;
use failure::{format_err, Error};
use log::{error, trace};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, Window};
use yew::prelude::*;
use yew::services::{
    resize::{ResizeTask, WindowDimensions},
    ResizeService,
};

pub enum Msg {
    Resize(WindowDimensions),
    Player(player::Response),
    Error(Error),
}

pub struct PlayerPage {
    link: ComponentLink<Self>,
    #[allow(dead_code)]
    player_agent: Box<dyn Bridge<player::PlayerAgent>>,
    window: Window,
    canvas: Option<HtmlCanvasElement>,
    render_context: Option<CanvasRenderingContext2d>,
    background_image: Option<HtmlImageElement>,
    background_loaded: bool,
    #[allow(dead_code)]
    resize_task: ResizeTask,
    width: u32,
    height: u32,
}

impl Component for PlayerPage {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
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
            canvas: None,
            render_context: None,
            background_image: None,
            background_loaded: false,
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
            Msg::Error(e) => {
                error!("ERROR: {}", e);
            }
            Msg::Player(_) => {}
        }
        false
    }

    fn mounted(&mut self) -> ShouldRender {
        if let Err(e) = self.on_mounted() {
            self.link.send_message(Msg::Error(e));
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <>
                <canvas id="player"/>
                <img id="player-background" src="player_background.png" />
            </>
        }
    }
}

impl PlayerPage {
    fn on_mounted(&mut self) -> Result<(), Error> {
        self.load_canvas()?;
        self.load_render_context()?;
        self.load_background_image()?;

        let dimensions = self.dimensions()?;
        self.resize(dimensions);

        Ok(())
    }

    fn load_canvas(&mut self) -> Result<(), Error> {
        let canvas = get_canvas(&self.window).ok_or_else(|| format_err!("Failed to get canvas"))?;
        self.canvas = Some(canvas);
        Ok(())
    }

    fn load_render_context(&mut self) -> Result<(), Error> {
        let context = self
            .canvas
            .as_ref()
            .ok_or_else(|| format_err!("Failed to get canvas"))?
            .get_context("2d")
            .map_err(|_| format_err!("Failed to get 2d rendering context"))?
            .ok_or_else(|| format_err!("Failed to get 2d rendering context"))?
            .unchecked_into();
        self.render_context = Some(context);
        Ok(())
    }

    fn load_background_image(&mut self) -> Result<(), Error> {
        let background_image = get_background_image(&self.window)
            .ok_or_else(|| format_err!("Failed to get background image"))?;
        self.background_image = Some(background_image);
        self.background_loaded = true;
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

        self.canvas.as_mut().unwrap().set_width(self.width);
        self.canvas.as_mut().unwrap().set_height(self.height);
        trace!("Canvas resized to: {}x{}", self.width, self.height);
    }
}

fn get_window() -> Window {
    web_sys::window().unwrap()
}

fn get_canvas(window: &Window) -> Option<HtmlCanvasElement> {
    Some(
        window
            .document()?
            .get_element_by_id("player")?
            .unchecked_into(),
    )
}

fn get_background_image(window: &Window) -> Option<HtmlImageElement> {
    Some(
        window
            .document()?
            .get_element_by_id("player-background")?
            .unchecked_into(),
    )
}
