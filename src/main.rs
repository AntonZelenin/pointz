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

use crate::drawer::render;
use crate::drawer::render::Rendering;
use crate::scene::{CameraState, Scene, Window, GUI};
use iced_wgpu::wgpu;
use iced_winit::winit::event_loop::EventLoop;
use scene::App;

pub fn main() {
    env_logger::init();
    shader::compile_shaders("src/shader");

    let event_loop = EventLoop::new();
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let window = Window::new(&instance, &event_loop);
    let scene = Scene::new();
    let mut rendering = Rendering::new(&instance, &window);
    rendering.add_drawer(Box::new(render::build_model_drawer(&rendering.device, &scene.model_data)));
    rendering.add_drawer(Box::new(render::build_debug_drawer(&rendering.device, &rendering.uniform_buffer)));
    let gui = GUI::new(&mut rendering.device, &window.viewport);
    let camera_state = CameraState::new(rendering.sc_desc.width, rendering.sc_desc.height);
    let mut app = App {
        window,
        rendering,
        gui,
        camera_state,
        scene,
    };

    event_loop.run(move |event, _, control_flow| {
        event::processor::process_events(&mut app, &event, control_flow)
    })
}
