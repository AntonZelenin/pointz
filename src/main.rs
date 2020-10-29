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
    // compile_shaders(
    //     "src/shader/shader.frag",
    //     "src/shader/frag.spv",
    //     shaderc::ShaderKind::Fragment,
    // );
    // compile_shaders(
    //     "src/shader/shader.vert",
    //     "src/shader/vert.spv",
    //     shaderc::ShaderKind::Vertex,
    // );
    // compile_shaders(
    //     "src/shader/light.vert",
    //     "src/shader/light_vert.spv",
    //     shaderc::ShaderKind::Vertex,
    // );
    // compile_shaders(
    //     "src/shader/light.frag",
    //     "src/shader/light_frag.spv",
    //     shaderc::ShaderKind::Fragment,
    // );

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let mut state = State::new(window);
    event_loop.run(move |event, _, control_flow| state.process_events(&event, control_flow))
}
