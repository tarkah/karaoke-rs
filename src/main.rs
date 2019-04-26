#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate crossbeam_channel;

extern crate self as karaoke;

mod channel;
mod collection;
mod config;
mod player;
mod queue;
mod site;
mod worker;

fn main() {
    player::run();
    worker::run();
    karaoke::site::run();
}
