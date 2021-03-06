use crate::renderer::buffer::Uniforms;
use crate::renderer::debug::DebugDrawer;
use crate::renderer::model::ModelDrawer;
use crate::model::{SimpleVertex};
use crate::app::GUI;
use crate::texture::Texture;
use crate::{renderer, model, object, texture};
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::util::DeviceExt;
use iced_wgpu::wgpu::{PipelineLayout, RenderPass, ShaderModule};
use iced_winit::futures;
use iced_winit::winit::dpi::PhysicalSize;
use iced_winit::winit::window::Window;
use std::collections::HashMap;
use std::iter;
use std::time::Instant;
use crate::renderer::bounding_sphere::BoundingSpheresDrawer;
// todo wgpu must be only inside the renderer, but that's not for sure

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

// todo are you sure it should be in rendering?
declare_handle!(MeshHandle, MaterialHandle, ObjectHandle);

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
    debug_drawer: DebugDrawer,
    bounding_spheres_drawer: Option<BoundingSpheresDrawer>,
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
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: Some(&surface),
                })
                .await
                .expect("Request adapter");

            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("device descriptor, I guess I have only one device"),
                        features: wgpu::Features::empty(),
                        limits: wgpu::Limits::default(),
                    },
                    None,
                )
                .await
                .expect("Failed to create device")
        });
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let depth_texture = Texture::create_depth_texture(&sc_desc, "depth_texture");
        let depth_texture_view = renderer::model::create_depth_view(&depth_texture, &device, &queue);
        let uniforms = Uniforms::new();
        // todo will update it later using camera
        // uniforms.update_view_proj(&camera, &projection);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            label: Some("uniform buffer"),
        });

        let model_drawer = ModelDrawer::new(&device);
        let debug_drawer = DebugDrawer::new(&device, &uniform_buffer);
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
            debug_drawer,
            bounding_spheres_drawer: None,
            depth_texture_view,
        }
    }

    // todo I need to wrap evey call to renderer, improve
    pub fn add_model(
        &mut self,
        model: &model::Model,
        instances: &Vec<object::Instance>,
    ) -> ObjectHandle {
        let object_handler = self.model_drawer.add_model(
            model,
            instances,
            &self.device,
            &self.queue,
            &self.uniform_buffer,
        );
        match &mut self.bounding_spheres_drawer {
            None => self.bounding_spheres_drawer = Some(BoundingSpheresDrawer::new(&self.device)),
            _ => {}
        }
        self.bounding_spheres_drawer.as_mut().unwrap().add(model, instances, &self.device, &self.uniform_buffer);
        object_handler
    }

    // todo add update all method?

    pub fn update_object(&mut self, object: &object::Object) {
        self.model_drawer.update_object(object, &self.queue);
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
                label: Some("Render pass"),
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
            // todo if I comment model renderer get frame will fail with timeout
            self.model_drawer.draw(&mut render_pass);
            self.debug_drawer.draw(&mut render_pass);
            if let Some(bounding_spheres_drawer) = &self.bounding_spheres_drawer {
                bounding_spheres_drawer.draw(&mut render_pass);
            }
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
        staging_belt.finish();

        // todo event to remove window from here?
        window.set_cursor_icon(iced_winit::conversion::mouse_interaction(mouse_interaction));
        self.queue.submit(iter::once(encoder.finish()));
    }

    pub fn add_line(&mut self, start: SimpleVertex, end: SimpleVertex) {
        self.debug_drawer.add_line(start, end, &self.queue);
    }
}

pub fn build_render_pipeline(
    device: &wgpu::Device,
    render_pipeline_layout: &PipelineLayout,
    vs_module: ShaderModule,
    fs_module: ShaderModule,
    vertex_buffer_layout: wgpu::VertexBufferLayout,
    topology: wgpu::PrimitiveTopology,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("main"),
        layout: Some(render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: "main",
            buffers: &[vertex_buffer_layout],
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: "main",
            targets: &[wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendState::REPLACE,
                alpha_blend: wgpu::BlendState::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
        }),
        primitive: wgpu::PrimitiveState {
            topology,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::Back,
            strip_index_format: None,
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: Default::default(),
            clamp_depth: false,
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
    })
}
