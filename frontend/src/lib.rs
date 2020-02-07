#![recursion_limit = "512"]
#![allow(clippy::eval_order_dependence)]

use log::{trace, Level};
use wasm_bindgen::prelude::*;
use wasm_logger::Config;

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
        Config::new(Level::Trace)
    } else {
        Config::new(Level::Info).module_prefix("karaoke_rs_frontend")
    };

    wasm_logger::init(log_config);

    trace!("Initializing yew...");
    yew::start_app::<app::Model>();
    Ok(())
}
