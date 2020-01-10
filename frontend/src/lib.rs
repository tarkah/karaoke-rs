#![recursion_limit = "512"]
#![allow(clippy::eval_order_dependence)]

use log::trace;
use wasm_bindgen::prelude::*;

mod app;
mod components;
mod model;
mod pages;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
    web_logger::init();

    trace!("Initializing yew...");
    yew::initialize();

    yew::start_app::<app::Model>();
    Ok(())
}
