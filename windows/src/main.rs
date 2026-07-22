#![allow(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod injector;
mod keyboard;
mod sender;
mod targets;

mod app;

use app::App;

fn main() {
    App::run();
}
