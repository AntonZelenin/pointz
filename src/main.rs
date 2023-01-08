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

// todo cursor_position should not be in gui
// todo cursor_watcher should not be in camera state
// todo move processing of all input events into one module
// todo you can load models in parallel, check learn-wgpu
// todo migrate to glam?
// todo implement "panels" in gui
pub fn main() {
    // todo define a ROOT const
    shader::compile_shaders("src/shader");
    App::run();
}
