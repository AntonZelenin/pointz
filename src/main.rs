mod controls;
mod scene;
mod camera;

use controls::Controls;
use scene::Scene;
use shaderc;
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, program, winit, Debug, Size};
use winit::{
    dpi::PhysicalPosition,
    event::{Event, ModifiersState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use std::fs;
use std::fs::File;
use std::io::{Write};

pub fn main() {
    compile_my_shader("src/shader/my.frag", "src/shader/my_frag.spv", shaderc::ShaderKind::Fragment);
    compile_my_shader("src/shader/my.vert", "src/shader/my_vert.spv", shaderc::ShaderKind::Vertex);

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let mut scene = Scene::new(&window);
    let mut resized = false;
    let mut modifiers = ModifiersState::default();
    let controls = Controls::new();

    let physical_size = window.inner_size();
    let mut viewport = Viewport::with_physical_size(
        Size::new(physical_size.width, physical_size.height),
        window.scale_factor(),
    );
    let mut cursor_position = PhysicalPosition::new(-1.0, -1.0);
    let mut debug = Debug::new();
    let mut renderer =
        Renderer::new(Backend::new(&mut scene.device, Settings::default()));
    let mut state = program::State::new(
        controls,
        viewport.logical_size(),
        conversion::cursor_position(cursor_position, viewport.scale_factor()),
        &mut renderer,
        &mut debug,
    );
    let mut last_render_time = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event, .. } => {
                if !scene.input(&event) {
                    match event {
                        WindowEvent::CursorMoved { position, .. } => {
                            cursor_position = position;
                        }
                        WindowEvent::ModifiersChanged(new_modifiers) => {
                            modifiers = new_modifiers;
                        }
                        WindowEvent::Resized(new_size) => {
                            viewport = Viewport::with_physical_size(
                                Size::new(new_size.width, new_size.height),
                                window.scale_factor(),
                            );
                            resized = true;
                        }
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        }
                        _ => {}
                    }
                }
                if let Some(event) = iced_winit::conversion::window_event(
                    &event,
                    window.scale_factor(),
                    modifiers,
                ) {
                    state.queue_event(event);
                }
            }
            Event::MainEventsCleared => {
                if !state.is_queue_empty() {
                    let _ = state.update(
                        viewport.logical_size(),
                        conversion::cursor_position(
                            cursor_position,
                            viewport.scale_factor(),
                        ),
                        None,
                        &mut renderer,
                        &mut debug,
                    );
                }
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                scene.update(dt);
                if resized {
                    scene.resize(window.inner_size());
                    resized = false;
                }
                let frame = scene.swap_chain.get_next_texture().expect("Next frame");
                let mut encoder = scene.device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { label: None },
                );
                let program = state.program();
                {
                    let mut render_pass = scene.clear(
                        &frame.view,
                        &mut encoder,
                        program.background_color(),
                    );
                    scene.draw(&mut render_pass);
                }
                let mouse_interaction = renderer.backend_mut().draw(
                    &mut scene.device,
                    &mut encoder,
                    &frame.view,
                    &viewport,
                    state.primitive(),
                    &debug.overlay(),
                );
                scene.queue.submit(&[encoder.finish()]);
                window.set_cursor_icon(
                    iced_winit::conversion::mouse_interaction(
                        mouse_interaction,
                    ),
                );
            }
            _ => {}
        }
    })
}

fn compile_my_shader(path: &str, out: &str, shader_type: shaderc::ShaderKind) {
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.add_macro_definition("EP", Some("main"));
    let source = fs::read_to_string(path).expect("file doesn't exist");
    let frag = compiler
        .compile_into_spirv(
            &source,
            shader_type,
            // "my.frag",
            path,
            "main",
            Some(&options),
        )
        .unwrap();
    let mut file = File::create(out).unwrap();
    file.write_all(frag.as_binary_u8()).unwrap();
}
