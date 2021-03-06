extern crate log;

use app::App;

mod camera;
mod renderer;
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
mod scene;

pub fn main() {
    shader::compile_shaders("src/shader");
    App::run();
}
