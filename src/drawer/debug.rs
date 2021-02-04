use crate::drawer::render::Drawer;
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::RenderPass;
use crate::drawer::render;
use iced_wgpu::wgpu::util::DeviceExt;
use crate::model::SimpleVertex;

pub struct DebugDrawer {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buff: wgpu::Buffer,
    pub index_buff: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

impl DebugDrawer {
    pub fn new(device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) -> DebugDrawer {
        let debug_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("debug_uniform_bind_group_layout"),
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &debug_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
            }],
            label: Some("debug_uniform_bind_group"),
        });
        let debug_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("debug pipeline"),
                bind_group_layouts: &[&debug_uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
            let vs_module =
                device.create_shader_module(wgpu::include_spirv!("../shader/spv/line.vert.spv"));
            let fs_module =
                device.create_shader_module(wgpu::include_spirv!("../shader/spv/line.frag.spv"));
            render::build_render_pipeline(device, &layout, vs_module, fs_module)
        };
        let vertex_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&[
                SimpleVertex {
                    position: [0.1, 0.1, 0.1],
                },
                SimpleVertex {
                    position: [5.3, 5.3, 5.3],
                },
            ]),
            usage: wgpu::BufferUsage::VERTEX,
        });
        const INDICES: &[u32] = &[0, 1];
        let index_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Debug Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsage::INDEX,
        });
        DebugDrawer {
            render_pipeline: debug_render_pipeline,
            vertex_buff,
            index_buff,
            uniform_bind_group,
        }
    }
}

impl Drawer for DebugDrawer {
    fn draw<'a: 'b, 'b>(&'a self, render_pass: &'b mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buff.slice(..));
        render_pass.set_index_buffer(self.index_buff.slice(..));
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.draw_indexed(0..2, 0, 0..1);
    }
}
