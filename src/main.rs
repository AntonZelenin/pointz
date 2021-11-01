extern crate log;

use app::App;

mod app;
mod camera;
mod editor;
mod event;
mod lighting;
mod model;
mod renderer;
mod scene;
mod shader;
mod texture;
mod widgets;

pub fn main() {
    // todo define a ROOT const
    shader::compile_shaders("src/shader");
    App::run();
}
