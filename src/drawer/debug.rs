use crate::drawer::render::Drawer;
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::{RenderPass, PipelineLayout, ShaderModule};
use iced_wgpu::wgpu::util::DeviceExt;
use crate::model::{SimpleVertex, Vertex};
use crate::texture;

pub struct DebugDrawer {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buff: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

impl DebugDrawer {
    pub fn new(device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) -> DebugDrawer {
        let debug_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
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
            build_render_pipeline(device, &layout, vs_module, fs_module)
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
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
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
    fn draw<'a: 'b, 'b>(&'a self, render_pass: &'b mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buff.slice(..));
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.draw(0..2, 0..1);
    }
}

pub fn build_render_pipeline(
    device: &wgpu::Device,
    render_pipeline_layout: &PipelineLayout,
    vs_module: ShaderModule,
    fs_module: ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("main"),
        layout: Some(render_pipeline_layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::Back,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        primitive_topology: wgpu::PrimitiveTopology::LineList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
            format: texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor::default(),
        }),
        vertex_state: wgpu::VertexStateDescriptor {
            // todo why 16
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[SimpleVertex::desc()],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    })
}
