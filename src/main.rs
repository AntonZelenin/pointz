extern crate log;

mod buffer;
mod camera;
mod controls;
mod event;
mod instance;
mod lighting;
mod model;
mod primitives;
mod scene;
mod texture;
mod widgets;
mod shader;

use iced_winit::winit::{event_loop::EventLoop, window::Window};
use scene::App;

pub fn main() {
    env_logger::init();
    shader::compile_shaders("src/shader");

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let mut app = App::new(window);
    event_loop.run(
        move |event, _, control_flow| event::processor::process_events(&mut app, &event, control_flow)
    )
}
