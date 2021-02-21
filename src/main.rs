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
