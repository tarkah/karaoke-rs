#![recursion_limit = "512"]
#![allow(clippy::eval_order_dependence)]

use log::{trace, Level};
use wasm_bindgen::prelude::*;
use web_logger::Config;

mod agents;
mod app;
mod components;
mod model;
mod pages;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
    let log_config = if cfg!(debug_assertions) {
        Config {
            level: Level::Trace,
        }
    } else {
        Config { level: Level::Info }
    };

    web_logger::custom_init(log_config);

    trace!("Initializing yew...");
    yew::initialize();

    yew::start_app::<app::Model>();
    Ok(())
}
