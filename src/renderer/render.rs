use crate::renderer::buffer::Uniforms;
use crate::renderer::debug::DebugDrawer;
use crate::renderer::model::ModelDrawer;
use crate::model::{SimpleVertex, Model};
use crate::texture::Texture;
use crate::{renderer, model, texture};
use crate::editor::GUI;
use crate::scene::manager::Object;
use wgpu::util::DeviceExt;
use iced_winit::futures;
use iced_winit::winit::dpi::PhysicalSize;
use iced_winit::winit::window::Window;
use std::iter;
use std::time::Instant;
// todo wgpu must be only inside the renderer, but that's not for sure

pub trait Drawer {
    fn draw<'a: 'b, 'b>(&'a self, render_pass: &'b mut wgpu::RenderPass<'a>);
}

pub struct InternalMesh {
    pub id: usize,
    pub count: usize,
    pub material_id: Option<usize>,
}

pub struct InternalModel {
    pub id: usize,
    pub num_of_instances: usize,
    pub internal_meshes: Vec<InternalMesh>,
}

pub struct RenderingState {
    pub gui: GUI,
    pub viewport: iced_wgpu::Viewport,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface,
    pub queue: wgpu::Queue,
    pub device: wgpu::Device,
    pub uniforms: Uniforms,
    pub uniform_buffer: wgpu::Buffer,
    pub last_render_time: Instant,
    model_drawer: ModelDrawer,
    debug_drawer: DebugDrawer,
    bounding_spheres_drawer: Option<ModelDrawer>,
    pub depth_texture_view: wgpu::TextureView,
}

impl RenderingState {
    pub fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface,
        size: PhysicalSize<u32>,
        scale_factor: f64,
    ) -> RenderingState {
        let (texture_format, (device, queue)) = futures::executor::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                })
                .await
                .expect("Request adapter");

            (
                surface
                    .get_preferred_format(&adapter)
                    .expect("Get preferred format"),
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
            )
        });
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &surface_config);
        let depth_texture = Texture::create_depth_texture(&surface_config, "depth_texture");
        let depth_texture_view = renderer::model::create_depth_view(&depth_texture, &device, &queue);
        let uniforms = Uniforms::new();
        // todo will update it later using camera
        // uniforms.update_view_proj(&camera, &projection);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            label: Some("uniform buffer"),
        });

        let model_drawer = ModelDrawer::new(&device, wgpu::PrimitiveTopology::TriangleList);
        let debug_drawer = DebugDrawer::new(&device, &uniform_buffer);
        let viewport = iced_wgpu::Viewport::with_physical_size(
            iced::Size::new(size.width, size.height),
            scale_factor,
        );
        let gui = GUI::new(&device, scale_factor, size, texture_format);

        RenderingState {
            gui,
            viewport,
            surface,
            surface_config,
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
    pub fn init_model(&mut self, model: &Model) {
        self.model_drawer.init_model(
            model,
            &self.device,
            &self.queue,
            &self.uniform_buffer,
        )
    }

    pub fn init_bounding_sphere_model(&mut self, model: &Model) {
        self.bounding_spheres_drawer.as_mut().unwrap().init_model(
            model,
            &self.device,
            &self.queue,
            &self.uniform_buffer,
        )
    }

    pub fn add_instances(
        &mut self,
        model: &model::Model,
        instances: &Vec<&Object>,
    ) {
        self.model_drawer.add_instances(
            model.id,
            instances,
            &self.device,
            &self.uniform_buffer,
            &self.queue
        );
    }

    pub fn init_bounding_sphere(&mut self, model: &Model) {
        match &mut self.bounding_spheres_drawer {
            None => {
                self.bounding_spheres_drawer = Some(ModelDrawer::new(&self.device, wgpu::PrimitiveTopology::LineList));
                self.init_bounding_sphere_model(model);
            },
            _ => {panic!("Bounding sphere already initialized")}
        }
    }

    pub fn add_bounding_sphere_instances(&mut self, bounding_model_id: usize, sphere_instances: &Vec<&Object>) {
        self.bounding_spheres_drawer.as_mut().unwrap().add_instances(bounding_model_id,sphere_instances, &self.device, &self.uniform_buffer, &self.queue);
    }

    // todo add update all method?

    pub fn update_object(&mut self, object: &Object) {
        self.model_drawer.update_object(object, &self.queue);
    }

    pub fn render(&mut self, window: &Window) {
        let frame = self
            .surface
            .get_current_texture()
            .expect("Timeout getting texture");
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let view = &frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view,
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
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
            view,
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
    render_pipeline_layout: &wgpu::PipelineLayout,
    vs_module: wgpu::ShaderModule,
    fs_module: wgpu::ShaderModule,
    vertex_buffer_layout: wgpu::VertexBufferLayout,
    topology: wgpu::PrimitiveTopology,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
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
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent::REPLACE,
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            }],
        }),
        primitive: wgpu::PrimitiveState {
            topology,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            strip_index_format: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
    })
}
