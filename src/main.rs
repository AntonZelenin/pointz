extern crate log;

mod buffer;
mod camera;
mod drawer;
mod editor;
mod event;
mod lighting;
mod model;
mod object;
mod primitives;
mod app;
mod shader;
mod texture;
mod widgets;

use app::App;

pub fn main() {
    // todo has anything changed?
    // env_logger::init();
    shader::compile_shaders("src/shader");
    App::run();
}
