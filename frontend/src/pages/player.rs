use crate::agents::player;
use failure::{format_err, Error};
use js_sys::JsString;
use log::{error, trace};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageBitmap, ImageData, Window};
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
    RenderFrame((ImageBitmap, (f32, f32, f32, f32))),
    Error(Error),
}

pub struct PlayerPage {
    link: ComponentLink<Self>,
    #[allow(dead_code)]
    player_agent: Box<dyn Bridge<player::PlayerAgent>>,
    window: Window,
    canvas: Option<HtmlCanvasElement>,
    render_context: Option<CanvasRenderingContext2d>,
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
            Msg::Player(response) => match response {
                player::Response::RenderFrame {
                    mut cdg_frame,
                    background,
                } => {
                    let callback = self.link.callback(Msg::RenderFrame);

                    let image: ImageData = ImageData::new_with_u8_clamped_array_and_sh(
                        wasm_bindgen::Clamped(cdg_frame.as_mut_slice()),
                        300,
                        216,
                    )
                    .unwrap();

                    let promise = self
                        .window
                        .create_image_bitmap_with_image_data(&image)
                        .unwrap();

                    spawn_local(async move {
                        let future = JsFuture::from(promise);
                        let image: ImageBitmap = future.await.unwrap().unchecked_into();
                        callback.emit((image, background))
                    });

                    drop(image);
                }
                player::Response::ClearCanvas => {
                    self.clear_canvas();
                }
            },
            Msg::RenderFrame((cdg_frame, background)) => {
                self.render_frame(cdg_frame, background);
            }
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

    fn render_frame(&mut self, cdg_frame: ImageBitmap, background: (f32, f32, f32, f32)) {
        let render_context = self.render_context.as_ref().unwrap();

        let color: JsString = format!(
            "rgba({}, {}, {}, {})",
            background.0, background.1, background.2, background.3
        )
        .as_str()
        .into();
        render_context.set_fill_style(&color);
        render_context.fill_rect(0.0, 0.0, self.width as f64, self.height as f64);

        let color: JsString = format!("rgba({}, {}, {}, {})", 0, 0, 0, 1).as_str().into();
        render_context.set_fill_style(&color);
        render_context.fill_rect(
            self.width as f64 / 2.0 - (300.0 * 1.5) / 2.0,
            self.height as f64 / 2.0 - (216.0 * 1.5) / 2.0,
            300.0 * 1.5,
            216.0 * 1.5,
        );

        let _ = render_context
            .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &cdg_frame,
                0.0,
                0.0,
                300.0,
                216.0,
                self.width as f64 / 2.0 - (300.0 * 1.5) / 2.0,
                self.height as f64 / 2.0 - (216.0 * 1.5) / 2.0,
                300.0 * 1.5,
                216.0 * 1.5,
            );

        cdg_frame.close();
    }

    fn clear_canvas(&mut self) {
        let render_context = self.render_context.as_ref().unwrap();
        render_context.clear_rect(0.0, 0.0, self.width as f64, self.height as f64);
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
