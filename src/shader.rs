use shaderc;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::ffi::OsStr;

pub fn compile_shaders(dir: &str) {
    let dir = Path::new(dir);
    compile_all(dir);
}

fn compile_all(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            compile_all(&path);
        } else {
            compile_shader(&path);
        }
    }
}

fn compile_shader(path: &Path) {
    let patasdfash = path.parent().unwrap().to_str().unwrap();
    let out = format!("{}.spv", path.file_name().and_then(OsStr::to_str).unwrap());
    let shader_type: shaderc::ShaderKind = get_shader_king(path.extension().and_then(OsStr::to_str).unwrap());
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.add_macro_definition("EP", Some("main"));
    let source = fs::read_to_string(path).expect("file doesn't exist");
    let frag = compiler
        .compile_into_spirv(&source, shader_type, path.to_str().unwrap(), "main", Some(&options))
        .unwrap();
    let mut file = File::create(out).unwrap();
    file.write_all(frag.as_binary_u8()).unwrap();
}

fn get_shader_king(extension: &str) -> shaderc::ShaderKind {
    match extension {
        "frag" => shaderc::ShaderKind::Fragment,
        "vert" => shaderc::ShaderKind::Vertex,
        _ => panic!("Unsupported shader extension provided"),
    }
}
