// #[macro_use]
extern crate log;

mod buffer;
mod camera;
mod controls;
mod instance;
mod model;
mod scene;
mod texture;
mod lighting;

use scene::State;
use shaderc;
use std::fs;
use std::fs::File;
use std::io::Write;
use winit::{event_loop::EventLoop, window::Window};

pub fn main() {
    env_logger::init();

    // compile_my_shader(
    //     "src/shader/shader.frag",
    //     "src/shader/frag.spv",
    //     shaderc::ShaderKind::Fragment,
    // );
    // compile_my_shader(
    //     "src/shader/shader.vert",
    //     "src/shader/vert.spv",
    //     shaderc::ShaderKind::Vertex,
    // );
    // compile_my_shader(
    //     "src/shader/light.vert",
    //     "src/shader/light_vert.spv",
    //     shaderc::ShaderKind::Vertex,
    // );
    // compile_my_shader(
    //     "src/shader/light.frag",
    //     "src/shader/light_frag.spv",
    //     shaderc::ShaderKind::Fragment,
    // );

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let mut state = State::new(window);
    event_loop.run(move |event, _, control_flow| state.process_events(&event, control_flow))
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
            // "shader.frag",
            path,
            "main",
            Some(&options),
        )
        .unwrap();
    let mut file = File::create(out).unwrap();
    file.write_all(frag.as_binary_u8()).unwrap();
}
