use std::fs;
use crate::renderer::render::Drawer;
use crate::model::{SimpleVertex, Vertex};
use wgpu;
use wgpu::util::DeviceExt;
use crate::renderer::render;

pub struct DebugDrawer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buff: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

impl DebugDrawer {
    pub fn new(device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) -> DebugDrawer {
        let debug_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
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
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("debug_uniform_bind_group"),
        });
        let debug_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("debug pipeline"),
                bind_group_layouts: &[&debug_uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
            let vs_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("line.vert"),
                source: wgpu::util::make_spirv(&fs::read("src/shader/spv/line.vert.spv").unwrap()),
            });
            let fs_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("line.frag"),
                source: wgpu::util::make_spirv(&fs::read("src/shader/spv/line.frag.spv").unwrap()),
            });
            render::build_render_pipeline(device, &layout, vs_module, fs_module, SimpleVertex::desc(), wgpu::PrimitiveTopology::LineList)
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
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        DebugDrawer {
            render_pipeline: debug_render_pipeline,
            vertex_buff,
            uniform_bind_group,
        }
    }

    pub fn add_line(&mut self, start: SimpleVertex, end: SimpleVertex, queue: &wgpu::Queue) {
        queue.write_buffer(&self.vertex_buff, 0, bytemuck::cast_slice(&[start, end]));
    }
}

impl Drawer for DebugDrawer {
    fn draw<'a: 'b, 'b>(&'a self, render_pass: &'b mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buff.slice(..));
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.draw(0..2, 0..1);
    }
}
