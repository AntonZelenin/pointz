extern crate log;

mod buffer;
mod camera;
mod drawer;
mod event;
mod object;
mod lighting;
mod model;
mod primitives;
mod scene;
mod shader;
mod texture;
mod widgets;
mod editor;

use scene::App;

pub fn main() {
    env_logger::init();
    shader::compile_shaders("src/shader");
    App::run();
}
