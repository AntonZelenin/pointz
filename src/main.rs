extern crate log;

mod buffer;
mod camera;
mod controls;
mod drawer;
mod event;
mod instance;
mod lighting;
mod model;
mod primitives;
mod scene;
mod shader;
mod texture;
mod widgets;

use scene::App;

pub fn main() {
    env_logger::init();
    shader::compile_shaders("src/shader");
    App::run();
}
