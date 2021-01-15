use crate::buffer::Uniforms;
use crate::lighting::Light;
use crate::model::{ModelVertex, SimpleVertex, Vertex};
use crate::scene::Window;
use crate::texture::Texture;
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::util::DeviceExt;
use iced_wgpu::wgpu::{PipelineLayout, RenderPass, ShaderModule};
use iced_winit::{futures, Color};
use std::time::Instant;
use crate::drawer;

pub trait Drawer {
    fn draw<'a>(&'a self, render_pass: &'a mut RenderPass<'a>);
}

pub struct DebugDrawer {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buff: wgpu::Buffer,
    pub index_buff: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

impl Drawer for DebugDrawer {
    fn draw<'a>(&'a self, render_pass: &'a mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buff.slice(..));
        render_pass.set_index_buffer(self.index_buff.slice(..));
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.draw_indexed(0..2, 0, 0..1);
    }
}

pub struct Rendering {
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub queue: wgpu::Queue,
    pub device: wgpu::Device,
    pub uniforms: Uniforms,
    pub uniform_buffer: wgpu::Buffer,
    pub depth_texture: Texture,
    pub last_render_time: Instant,
    drawers: Vec<Box<dyn Drawer>>,
}

impl Rendering {
    pub fn new(instance: &wgpu::Instance, window: &Window) -> Rendering {
        let (mut device, queue) = futures::executor::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::Default,
                    compatible_surface: Some(&window.surface),
                })
                .await
                .expect("Request adapter");

            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        features: wgpu::Features::empty(),
                        limits: wgpu::Limits::default(),
                        shader_validation: true,
                    },
                    None,
                )
                .await
                .expect("Failed to create device")
        });
        let size = window.window.inner_size();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&window.surface, &sc_desc);
        let depth_texture = Texture::create_depth_texture(&device, &sc_desc, "depth_texture");
        let mut uniforms = Uniforms::new();
        // todo will update it later using camera
        // uniforms.update_view_proj(&camera, &projection);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            label: Some("uniform buffer"),
        });

        Rendering {
            swap_chain,
            sc_desc,
            queue,
            device,
            depth_texture,
            last_render_time: std::time::Instant::now(),
            uniforms,
            uniform_buffer,
            drawers: Vec::new(),
        }
    }

    // todo render self of pass renderable and render it?
    pub fn render(&mut self) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting texture")
            .output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: {
                        let [r, g, b, a] = Color::BLACK.into_linear();
                        wgpu::LoadOp::Clear(wgpu::Color {
                            r: r as f64,
                            g: g as f64,
                            b: b as f64,
                            a: a as f64,
                        })
                    },
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                attachment: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: true,
                }),
            }),
        });

        for drawer in self.drawers {
            drawer.draw(&mut render_pass);
        }
    }

    pub fn add_drawer(&mut self, drawer: Box<dyn Drawer>) {
        self.drawers.push(drawer);
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
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
            // todo keep this const in a texture as it used to be?
            format: drawer::model::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor::default(),
        }),
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[ModelVertex::desc()],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    })
}

pub fn build_debug_drawer(device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) -> DebugDrawer {
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
    let debug_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
            device.create_shader_module(wgpu::include_spirv!("../shader/spv/debug.vert.spv"));
        let fs_module =
            device.create_shader_module(wgpu::include_spirv!("../shader/spv/debug.frag.spv"));
        build_render_pipeline(device, &layout, vs_module, fs_module)
    };
    let debug_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&[
            SimpleVertex {
                position: [-30.0, 23.0, 25.0],
            },
            SimpleVertex {
                position: [256.0, -918.0, 302.0],
            },
        ]),
        usage: wgpu::BufferUsage::VERTEX,
    });
    const INDICES: &[u32] = &[0, 1];
    let debug_index_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Debug Index Buffer"),
        contents: bytemuck::cast_slice(INDICES),
        usage: wgpu::BufferUsage::INDEX,
    });
    // todo one structure with optional fields?
    DebugDrawer {
        render_pipeline: debug_render_pipeline,
        vertex_buff: debug_buff,
        index_buff: debug_index_buff,
        uniform_bind_group: debug_uniform_bind_group,
    }
}
