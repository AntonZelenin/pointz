use iced_wgpu::wgpu;

pub const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-2.5, 0.0, 0.0],
        tex_coords: [0.4, 0.0],
    },
    Vertex {
        position: [2.5, 0.0, 0.0],
        tex_coords: [0.8, 0.0],
    },
    Vertex {
        position: [0.0, 2.5, -2.3],
        tex_coords: [0.5, 0.8],
    },
    Vertex {
        position: [0.0, 0.0, -2.5],
        tex_coords: [0.0, 0.0],
    },
];

pub const INDICES: &[u16] = &[2, 1, 0, 0, 1, 3, 1, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
            ],
        }
    }
}
