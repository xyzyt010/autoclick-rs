#![allow(unsafe_code)]

mod app;
mod detect;
mod engine;
mod injector;
mod keyboard;
mod targets;

use app::App;

fn main() {
    App::run();
}
