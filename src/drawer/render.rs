use iced_wgpu::wgpu;
use crate::scene::Window;
use crate::texture::Texture;
use iced_winit::{futures, Color};
use std::time::Instant;
use crate::buffer::Uniforms;
use crate::lighting::Light;
use iced_wgpu::wgpu::util::DeviceExt;
use crate::model::{ModelData, Model, Mesh, Material, ModelVertex, Vertex, SimpleVertex};
use iced_wgpu::wgpu::{PipelineLayout, ShaderModule, RenderPass};
use std::ops::Range;

pub trait Drawer {
    fn draw<'a>(&'a self, render_pass: &'a mut RenderPass<'a>);
}

pub trait Renderable {
    fn get_drawers(&self, device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) -> Vec<Box<dyn Drawer + '_>>;
}

pub struct ModelDrawer<'a> {
    pub render_pipeline: &'a wgpu::RenderPipeline,
    pub model_data: &'a Vec<ModelData>,
    pub light_bind_group: &'a wgpu::BindGroup,
}

impl<'a> ModelDrawer<'a> {
    fn draw_model_instanced<'b>(
        &'b self,
        render_pass: &'b mut RenderPass<'b>,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(render_pass, mesh, material, instances.clone(), uniforms, light);
        }
    }

    fn draw_mesh_instanced<'b>(
        &'b self,
        render_pass: &'b mut RenderPass<'b>,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        uniform_bind_group: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..));
        render_pass.set_bind_group(0, &uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &material.bind_group, &[]);
        render_pass.set_bind_group(2, &light, &[]);
        render_pass.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}

impl<'a> Drawer for ModelDrawer<'a> {
    fn draw<'b>(&'b self, render_pass: &'b mut RenderPass<'b>) {
        render_pass.set_pipeline(&self.render_pipeline);
        for model_data in self.model_data {
            self.draw_model_instanced(
                render_pass,
                &model_data.model,
                0..model_data.instances.len() as u32,
                &model_data.uniform_bind_group,
                &self.light_bind_group,
            );
        }
    }
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

pub struct Rendering<D: Drawer> {
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,

    pub queue: wgpu::Queue,
    pub device: wgpu::Device,

    pub render_pipeline: wgpu::RenderPipeline,
    pub light_render_pipeline: wgpu::RenderPipeline,

    pub uniforms: Uniforms,
    pub uniform_buffer: wgpu::Buffer,

    pub light_bind_group: wgpu::BindGroup,

    // drawers: Vec<Box<dyn Drawer>>,
    drawers: Vec<D>,

    // todo move
    pub depth_texture: Texture,
    pub last_render_time: Instant,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl<D: Drawer> Rendering<D> {
    pub fn new(instance: &wgpu::Instance, window: &Window) -> Rendering<D> {
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

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: true,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("uniform_bind_group_layout"),
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            component_type: wgpu::TextureComponentType::Float,
                            dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                    // normal map
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            component_type: wgpu::TextureComponentType::Float,
                            dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let light = Light::new((2.0, 2.0, 2.0).into(), (1.0, 1.0, 1.0).into());
        // We'll want to update our lights position, so we use COPY_DST
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[light]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            label: Some("light buffer"),
        });
        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(light_buffer.slice(..)),
            }],
            label: None,
        });
        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("light pipeline"),
                bind_group_layouts: &[&uniform_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });
            let vs_module =
                device.create_shader_module(wgpu::include_spirv!("../shader/spv/light.vert.spv"));
            let fs_module =
                device.create_shader_module(wgpu::include_spirv!("../shader/spv/light.frag.spv"));
            build_render_pipeline(&device, &layout, vs_module, fs_module)
        };

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
            render_pipeline,
            queue,
            device,
            depth_texture,
            last_render_time: std::time::Instant::now(),
            light_render_pipeline,
            uniforms,
            uniform_buffer,
            light_bind_group,
            drawers: Vec::new(),
            texture_bind_group_layout,
            uniform_bind_group_layout,
        }
    }

    fn build_model_drawer() -> ModelDrawer {

        let render_pipeline = {
            let render_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[
                        &uniform_bind_group_layout,
                        &texture_bind_group_layout,
                        &light_bind_group_layout,
                    ],
                    label: Some("main"),
                    push_constant_ranges: &[],
                });
            let vs_module = device.create_shader_module(wgpu::include_spirv!("../shader/spv/shader.vert.spv"));
            let fs_module = device.create_shader_module(wgpu::include_spirv!("../shader/spv/shader.frag.spv"));
            build_render_pipeline(&device, &render_pipeline_layout, vs_module, fs_module)
        };
        ModelDrawer {
            render_pipeline: &self.render_pipeline,
            model_data: &self.scene.model_data,
            light_bind_group: &self.light_bind_group,
        }
    }

    pub fn render<R: Renderable>(&mut self, renderable: R) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting texture")
            .output;

        let mut encoder =
            self
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

    // todo does D has Drawer bound?
    pub fn add_drawer(&mut self, drawer: D) {
        self.drawers.push(drawer);
    }

    fn get_drawers(&self) -> Vec<Box<dyn Drawer + '_>> {
        let mut drawers: Vec<Box<dyn Drawer>> = Vec::new();
        // todo bad idea to create bind group and pipeline every frame
        let debug_uniform_bind_group_layout =
            self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let debug_uniform_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &debug_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(self.uniform_buffer.slice(..)),
            }],
            label: Some("debug_uniform_bind_group"),
        });
        let debug_render_pipeline = {
            let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("debug pipeline"),
                bind_group_layouts: &[&debug_uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
            let vs_module =
                self.device.create_shader_module(wgpu::include_spirv!("../shader/spv/debug.vert.spv"));
            let fs_module =
                self.device.create_shader_module(wgpu::include_spirv!("../shader/spv/debug.frag.spv"));
            build_render_pipeline(&self.device, &layout, vs_module, fs_module)
        };
        let debug_buff = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
        let debug_index_buff = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Debug Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsage::INDEX,
        });
        drawers.push(Box::new(
            DebugDrawer {
                render_pipeline: debug_render_pipeline,
                // todo why should I keep it in rendering if it's needed only in the debug drawer? check similar
                vertex_buff: debug_buff,
                index_buff:debug_index_buff,
                uniform_bind_group: debug_uniform_bind_group,
            }
        ));
        drawers
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
            format: Texture::DEPTH_FORMAT,
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
