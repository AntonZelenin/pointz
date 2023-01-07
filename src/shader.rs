use shaderc;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

const SPV_PATH: &str = "src/shader/spv";

pub fn compile_shaders(dir: &str) {
    let dir = Path::new(dir);
    compile_all(dir);
}

fn compile_all(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let file_path = entry.path();
        if file_path.is_dir() {
            compile_all(&file_path);
        } else {
            if file_path.extension().unwrap() != "spv" {
                compile_shader(&file_path);
            }
        }
    }
}

fn compile_shader(file_path: &Path) {
    let out = format!(
        "{}/{}.spv",
        SPV_PATH,
        file_path.file_name().and_then(OsStr::to_str).unwrap()
    );
    let shader_type = get_shader_king(file_path.extension().and_then(OsStr::to_str).unwrap());
    let compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.add_macro_definition("EP", Some("main"));
    let source = fs::read_to_string(file_path).expect("file doesn't exist");
    let frag = compiler
        .compile_into_spirv(
            &source,
            shader_type,
            file_path.to_str().unwrap(),
            "main",
            Some(&options),
        )
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
