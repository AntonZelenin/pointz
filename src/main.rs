extern crate log;

mod buffer;
mod camera;
mod controls;
mod instance;
mod lighting;
mod model;
mod primitives;
mod scene;
mod texture;
mod widgets;
mod shader;

use iced_winit::winit::{event_loop::EventLoop, window::Window};
use scene::State;

pub fn main() {
    env_logger::init();
    shader::compile_shaders("src/shader");

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let mut state = State::new(window);
    event_loop.run(move |event, _, control_flow| state.process_events(&event, control_flow))
}
