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
mod shader;
mod drawer;
mod texture;
mod widgets;

use iced_wgpu::wgpu;
use iced_winit::winit::event_loop::EventLoop;
use scene::App;
use crate::scene::{CameraState, Scene, Window, GUI};
use crate::drawer::render::Rendering;

pub fn main() {
    env_logger::init();
    shader::compile_shaders("src/shader");

    let event_loop = EventLoop::new();
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let window = Window::new(&instance, &event_loop);
    let mut rendering = Rendering::new(&instance, &window);
    let gui = GUI::new(&mut rendering.device, &window.viewport);
    let camera_state = CameraState::new(&rendering.sc_desc);
    let scene = Scene::new(
        &rendering.device,
        &rendering.queue,
        &rendering.texture_bind_group_layout,
        &rendering.uniform_bind_group_layout,
        &rendering.uniform_buffer,
    );
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
