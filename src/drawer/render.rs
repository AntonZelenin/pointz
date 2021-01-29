use crate::buffer::Uniforms;
use crate::drawer::model::{ModelDrawer, Object};
use crate::model::{ModelVertex, Vertex};
use crate::scene::GUI;
use crate::texture::Texture;
use crate::{drawer, instance, model, texture};
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::util::DeviceExt;
use iced_wgpu::wgpu::{PipelineLayout, RenderPass, ShaderModule};
use iced_winit::futures;
use iced_winit::winit::dpi::PhysicalSize;
use iced_winit::winit::window::Window;
use std::collections::HashMap;
use std::iter;
use std::time::Instant;

#[macro_export]
macro_rules! declare_handle {
    ($($name:ident),*) => {$(
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name(pub(crate) usize);

        impl $name {
            pub fn get(&self) -> usize {
                self.0
            }
        }
    )*};
}

declare_handle!(MeshHandle, MaterialHandle, ObjectHandle, InstanceHandle);

pub struct ResourceRegistry<T> {
    mapping: HashMap<usize, T>,
}

impl<T> ResourceRegistry<T> {
    pub fn new() -> ResourceRegistry<T> {
        ResourceRegistry {
            mapping: HashMap::new(),
        }
    }

    pub fn insert(&mut self, handle: usize, data: T) {
        self.mapping.insert(handle, data);
    }

    pub fn get(&self, handle: usize) -> &T {
        self.mapping.get(&handle).unwrap()
    }
}

pub trait Drawer {
    fn draw<'a: 'b, 'b>(&'a self, render_pass: &'b mut RenderPass<'a>);
}

// pub struct DebugDrawer {
//     pub render_pipeline: wgpu::RenderPipeline,
//     pub vertex_buff: wgpu::Buffer,
//     pub index_buff: wgpu::Buffer,
//     pub uniform_bind_group: wgpu::BindGroup,
// }
//
// impl Drawer for DebugDrawer {
//     fn draw<'a: 'b, 'b>(
//         &'a self,
//         render_pass: &'b mut RenderPass<'a>,
//         _: &Vec<model::ModelBatch>
//     ) {
//         render_pass.set_pipeline(&self.render_pipeline);
//         render_pass.set_vertex_buffer(0, self.vertex_buff.slice(..));
//         render_pass.set_index_buffer(self.index_buff.slice(..));
//         render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
//         render_pass.draw_indexed(0..2, 0, 0..1);
//     }
// }

pub struct RenderingState {
    pub gui: GUI,
    pub viewport: iced_wgpu::Viewport,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub surface: wgpu::Surface,
    pub queue: wgpu::Queue,
    pub device: wgpu::Device,
    pub uniforms: Uniforms,
    pub uniform_buffer: wgpu::Buffer,
    pub last_render_time: Instant,
    model_drawer: ModelDrawer,
    // debug_drawer: DebugDrawer,
    pub depth_texture_view: wgpu::TextureView,
}

impl RenderingState {
    pub fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface,
        size: PhysicalSize<u32>,
        scale_factor: f64,
    ) -> RenderingState {
        let (device, queue) = futures::executor::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::Default,
                    compatible_surface: Some(&surface),
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
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let depth_texture = Texture::create_depth_texture(&sc_desc, "depth_texture");
        let depth_texture_view = drawer::model::create_depth_view(&depth_texture, &device, &queue);
        let uniforms = Uniforms::new();
        // todo will update it later using camera
        // uniforms.update_view_proj(&camera, &projection);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            label: Some("uniform buffer"),
        });

        let model_drawer = ModelDrawer::build_model_drawer(&device);
        // let debug_drawer = render::build_debug_drawer(&device, &uniform_buffer);
        let viewport = iced_wgpu::Viewport::with_physical_size(
            iced::Size::new(size.width, size.height),
            scale_factor,
        );
        let gui = GUI::new(&device, scale_factor, size);

        RenderingState {
            gui,
            viewport,
            swap_chain,
            sc_desc,
            surface,
            queue,
            device,
            last_render_time: std::time::Instant::now(),
            uniforms,
            uniform_buffer,
            model_drawer,
            // debug_drawer,
            // todo do I need depth_texture here?
            depth_texture_view,
        }
    }

    // todo I need to wrap evey call to drawer, improve
    pub fn add_model(
        &mut self,
        model: model::Model,
        instances: Vec<instance::Instance>,
    ) -> Object {
        self.model_drawer.add_model(
            model,
            instances,
            &self.device,
            &self.queue,
            &self.uniform_buffer,
        )
    }

    pub fn update_instance(&mut self, handle: ObjectHandle, instance_idx: usize, instance: &instance::Instance) {
        self.model_drawer.update_instance(handle, instance_idx, instance, &self.queue);
    }

    pub fn render(&mut self, window: &Window) {
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
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: {
                            let [r, g, b, a] = self
                                .gui
                                .program_state
                                .program()
                                .background_color()
                                .into_linear();
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
                    attachment: &self.depth_texture_view,
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
            self.model_drawer.draw(&mut render_pass);
        }

        let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
        let mouse_interaction = self.gui.renderer.backend_mut().draw(
            &mut self.device,
            &mut staging_belt,
            &mut encoder,
            &frame.view,
            &self.viewport,
            self.gui.program_state.primitive(),
            &self.gui.debug.overlay(),
        );
        // todo event to remove window from here?
        window.set_cursor_icon(iced_winit::conversion::mouse_interaction(mouse_interaction));
        staging_belt.finish();

        // self.debug_drawer.draw(&mut render_pass, model_data);
        self.queue.submit(iter::once(encoder.finish()));
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
            format: texture::DEPTH_FORMAT,
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

// pub fn build_debug_drawer(device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) -> DebugDrawer {
//     let debug_uniform_bind_group_layout =
//         device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//             entries: &[wgpu::BindGroupLayoutEntry {
//                 binding: 0,
//                 visibility: wgpu::ShaderStage::VERTEX,
//                 ty: wgpu::BindingType::UniformBuffer {
//                     dynamic: false,
//                     min_binding_size: None,
//                 },
//                 count: None,
//             }],
//             label: Some("debug_uniform_bind_group_layout"),
//         });
//     let debug_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//         layout: &debug_uniform_bind_group_layout,
//         entries: &[wgpu::BindGroupEntry {
//             binding: 0,
//             resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
//         }],
//         label: Some("debug_uniform_bind_group"),
//     });
//     let debug_render_pipeline = {
//         let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//             label: Some("debug pipeline"),
//             bind_group_layouts: &[&debug_uniform_bind_group_layout],
//             push_constant_ranges: &[],
//         });
//         let vs_module =
//             device.create_shader_module(wgpu::include_spirv!("../shader/spv/debug.vert.spv"));
//         let fs_module =
//             device.create_shader_module(wgpu::include_spirv!("../shader/spv/debug.frag.spv"));
//         build_render_pipeline(device, &layout, vs_module, fs_module)
//     };
//     let debug_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//         label: Some("Vertex Buffer"),
//         contents: bytemuck::cast_slice(&[
//             SimpleVertex {
//                 position: [-30.0, 23.0, 25.0],
//             },
//             SimpleVertex {
//                 position: [256.0, -918.0, 302.0],
//             },
//         ]),
//         usage: wgpu::BufferUsage::VERTEX,
//     });
//     const INDICES: &[u32] = &[0, 1];
//     let debug_index_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//         label: Some("Debug Index Buffer"),
//         contents: bytemuck::cast_slice(INDICES),
//         usage: wgpu::BufferUsage::INDEX,
//     });
//     DebugDrawer {
//         render_pipeline: debug_render_pipeline,
//         vertex_buff: debug_buff,
//         index_buff: debug_index_buff,
//         uniform_bind_group: debug_uniform_bind_group,
//     }
// }
