use crate::buffer::Uniforms;
use crate::camera::{Camera, CameraController, Projection};
use crate::controls::GUI;
use crate::instance::{Instance, INSTANCE_DISPLACEMENT, NUM_INSTANCES_PER_ROW, NUM_ROWS};
use crate::model;
use crate::model::{Vertex, DrawModel, Model};
use crate::texture::Texture;
use cgmath::prelude::*;
use cgmath::{Deg, Matrix4, Point3, Quaternion, Rad, Vector3};
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::{PipelineLayout, ShaderModule};
use iced_wgpu::{Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Debug, Size};
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::{dpi::PhysicalPosition, event::ModifiersState, window::Window};
use crate::lighting::{Light, DrawLight};

const KEEP_CURSOR_POS_FOR_NUM_FRAMES: usize = 3;

pub struct State {
    viewport: Viewport,
    surface: wgpu::Surface,
    window: Window,
    // todo can be moved to a getter
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    render_pipeline: wgpu::RenderPipeline,
    queue: wgpu::Queue,
    device: wgpu::Device,
    renderer: Renderer,
    program_state: program::State<GUI>,
    depth_texture: Texture,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    obj_model: Model,
    uniforms: Uniforms,
    uniform_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    camera: Camera,
    projection: Projection,
    camera_controller: CameraController,
    last_frames_cursor_deltas: Vec<(f64, f64)>,
    camera_mode: bool,
    modifiers: ModifiersState,
    cursor_position: PhysicalPosition<f64>,
    resized: bool,
    last_render_time: Instant,
    debug: Debug,
    light: Light,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,
}

impl State {
    pub fn new(window: winit::window::Window) -> State {
        let surface = wgpu::Surface::create(&window);
        let (mut device, queue) = futures::executor::block_on(async {
            let adapter = wgpu::Adapter::request(
                &wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::Default,
                    compatible_surface: Some(&surface),
                },
                wgpu::BackendBit::PRIMARY,
            )
            .await
            .expect("Request adapter");

            adapter
                .request_device(&wgpu::DeviceDescriptor {
                    extensions: wgpu::Extensions {
                        anisotropic_filtering: false,
                    },
                    limits: wgpu::Limits::default(),
                })
                .await
        });
        let size = window.inner_size();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let camera = Camera::new(Point3::new(-30.0, 25.0, 25.0), Deg(0.0), Deg(-40.0));
        let projection = Projection::new(sc_desc.width, sc_desc.height, Deg(50.0), 0.1, 1000.0);

        let instances = (0..NUM_ROWS)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let position = Vector3 {
                        x: (x * 6) as f32,
                        y: 0.0,
                        z: (z * 6) as f32,
                    } - INSTANCE_DISPLACEMENT;
                    let rotation = if position.is_zero() {
                        // this is needed so an object at (0, 0, 0) won't get scaled to zero
                        // as Quaternions can effect scale if they're not created correctly
                        Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0))
                    } else {
                        Quaternion::from_axis_angle(position.clone().normalize(), Deg(45.0))
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer_size = instance_data.len() * std::mem::size_of::<Matrix4<f32>>();
        let instance_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&instance_data),
            wgpu::BufferUsage::STORAGE_READ | wgpu::BufferUsage::COPY_DST,
        );

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera, &projection);
        let uniform_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[uniforms]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: true,
                        },
                    },
                ],
                label: Some("uniform_bind_group_layout"),
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &uniform_buffer,
                        range: 0..std::mem::size_of_val(&uniforms) as wgpu::BufferAddress,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &instance_buffer,
                        range: 0..instance_buffer_size as wgpu::BufferAddress,
                    },
                },
            ],
            label: Some("uniform_bind_group"),
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            // component_type: wgpu::TextureComponentType::Uint,
                            component_type: wgpu::TextureComponentType::Float,
                            dimension: wgpu::TextureViewDimension::D2,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
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
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let light = Light::new((2.0, 2.0, 2.0).into(), (1.0, 1.0, 1.0).into());
        // We'll want to update our lights position, so we use COPY_DST
        let light_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[light]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
                label: None,
            });
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &light_buffer,
                    range: 0..std::mem::size_of_val(&light) as wgpu::BufferAddress,
                },
            }],
            label: None,
        });

        let physical_size = window.inner_size();
        let viewport = Viewport::with_physical_size(
            Size::new(physical_size.width, physical_size.height),
            window.scale_factor(),
        );

        let mut debug = Debug::new();
        let mut renderer = iced_wgpu::Renderer::new(Backend::new(&mut device, Settings::default()));
        let state = program::State::new(
            GUI::new(),
            viewport.logical_size(),
            conversion::cursor_position(PhysicalPosition::new(-1.0, -1.0), viewport.scale_factor()),
            &mut renderer,
            &mut debug,
        );

        let depth_texture = Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

        let (obj_model, command_buffers) = model::Model::load(
            &device,
            &texture_bind_group_layout,
            "resources/cube.obj",
        ).unwrap();

        queue.submit(&command_buffers);

        let render_pipeline = {
            let render_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[
                        &uniform_bind_group_layout,
                        &texture_bind_group_layout,
                        &light_bind_group_layout,
                    ],
                });
            let vs = include_bytes!("shader/vert.spv");
            let fs = include_bytes!("shader/frag.spv");
            let vs_module =
                device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&vs[..])).unwrap());
            let fs_module =
                device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&fs[..])).unwrap());
            build_render_pipeline(&device, &render_pipeline_layout, vs_module, fs_module)
        };

        let
            light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &uniform_bind_group_layout,
                    &light_bind_group_layout,
                ]
            });
            let vs = include_bytes!("shader/light_vert.spv");
            let fs = include_bytes!("shader/light_frag.spv");
            let vs_module =
                device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&vs[..])).unwrap());
            let fs_module =
                device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&fs[..])).unwrap());
            build_render_pipeline(
                &device,
                &layout,
                vs_module,
                fs_module,
            )
        };

        State {
            viewport,
            surface,
            window,
            swap_chain,
            sc_desc,
            render_pipeline,
            queue,
            device,
            renderer,
            program_state: state,
            depth_texture,
            instances,
            instance_buffer,
            obj_model,
            uniforms,
            uniform_bind_group,
            uniform_buffer,
            camera,
            projection,
            camera_controller: CameraController::new(4.0, 0.4),
            last_frames_cursor_deltas: Vec::with_capacity(3),
            camera_mode: false,
            modifiers: ModifiersState::default(),
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            resized: false,
            last_render_time: std::time::Instant::now(),
            debug,
            light,
            light_buffer,
            light_bind_group,
            light_render_pipeline,
        }
    }

    fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.light_render_pipeline);
        render_pass.draw_light_model(
            &self.obj_model,
            &self.uniform_bind_group,
            &self.light_bind_group,
        );
        render_pass.set_pipeline(&self.render_pipeline);
        // let mesh = &self.obj_model.meshes[0];
        // let material = &self.obj_model.materials[mesh.material];
        // render_pass.draw_mesh_instanced(&mesh,  material,0..self.instances.len() as _, &self.uniform_bind_group);
        render_pass.draw_model_instanced(
            &self.obj_model,
            0..self.instances.len() as u32,
            &self.uniform_bind_group,
            &self.light_bind_group,
        );
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.projection.resize(new_size.width, new_size.height);
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.depth_texture =
            Texture::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn process_events(&mut self, event: &Event<()>, control_flow: &mut ControlFlow) {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(key),
                                state,
                                ..
                            },
                        ..
                    } => {
                        self.camera_controller.process_keyboard(*key, *state);
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        self.camera_controller.process_scroll(delta);
                    }
                    WindowEvent::MouseInput {
                        button: MouseButton::Right,
                        state,
                        ..
                    } => {
                        self.camera_mode = *state == ElementState::Pressed;
                        self.window.set_cursor_visible(!self.camera_mode);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if self.camera_mode {
                            // make cursor stay at the same place
                            self.window
                                .set_cursor_position(self.cursor_position)
                                .unwrap();
                        } else {
                            self.cursor_position = *position;
                        }
                    }
                    WindowEvent::ModifiersChanged(new_modifiers) => {
                        self.modifiers = *new_modifiers;
                    }
                    WindowEvent::Resized(new_size) => {
                        self.viewport = Viewport::with_physical_size(
                            Size::new(new_size.width, new_size.height),
                            self.window.scale_factor(),
                        );
                        self.resized = true;
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
                if let Some(event) = iced_winit::conversion::window_event(
                    &event,
                    self.window.scale_factor(),
                    self.modifiers,
                ) {
                    self.program_state.queue_event(event);
                }
            }
            Event::MainEventsCleared => {
                if !self.program_state.is_queue_empty() {
                    let _ = self.program_state.update(
                        self.viewport.logical_size(),
                        conversion::cursor_position(
                            self.cursor_position,
                            self.viewport.scale_factor(),
                        ),
                        None,
                        &mut self.renderer,
                        &mut self.debug,
                    );
                }
                self.window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let now = std::time::Instant::now();
                let dt = now - self.last_render_time;
                self.last_render_time = now;
                self.update(dt);
                if self.resized {
                    self.resize(self.window.inner_size());
                    self.resized = false;
                }
                let frame = self.swap_chain.get_next_texture().expect("Next frame");
                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                let program = self.program_state.program();
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            load_op: wgpu::LoadOp::Clear,
                            store_op: wgpu::StoreOp::Store,
                            clear_color: {
                                let [r, g, b, a] = program.background_color().into_linear();
                                wgpu::Color {
                                    r: r as f64,
                                    g: g as f64,
                                    b: b as f64,
                                    a: a as f64,
                                }
                            },
                        }],
                        depth_stencil_attachment: Some(
                            wgpu::RenderPassDepthStencilAttachmentDescriptor {
                                attachment: &self.depth_texture.view,
                                depth_load_op: wgpu::LoadOp::Clear,
                                depth_store_op: wgpu::StoreOp::Store,
                                clear_depth: 1.0,
                                stencil_load_op: wgpu::LoadOp::Clear,
                                stencil_store_op: wgpu::StoreOp::Store,
                                clear_stencil: 0,
                            },
                        ),
                    });
                    self.draw(&mut render_pass);
                }
                let mouse_interaction = self.renderer.backend_mut().draw(
                    &mut self.device,
                    &mut encoder,
                    &frame.view,
                    &self.viewport,
                    self.program_state.primitive(),
                    &self.debug.overlay(),
                );
                self.queue.submit(&[encoder.finish()]);
                self.window
                    .set_cursor_icon(iced_winit::conversion::mouse_interaction(mouse_interaction));
            }
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    if self.last_frames_cursor_deltas.len() > KEEP_CURSOR_POS_FOR_NUM_FRAMES {
                        self.last_frames_cursor_deltas.drain(..1);
                    }
                    self.last_frames_cursor_deltas.push(*delta);
                    let (mouse_dx, mouse_dy) = self.get_avg_cursor_pos();
                    if self.camera_mode {
                        self.camera_controller
                            .process_mouse(mouse_dx / 2.0, mouse_dy / 2.0);
                    }
                }
                _ => {}
            },
            _ => {}
        };
    }

    fn get_avg_cursor_pos(&self) -> (f64, f64) {
        let mut avg_dx = 0.0;
        let mut avg_dy = 0.0;
        for (x, y) in self.last_frames_cursor_deltas.iter() {
            avg_dx += x;
            avg_dy += y;
        }
        let size = self.last_frames_cursor_deltas.len() as f64;
        (avg_dx / size, avg_dy / size)
    }

    fn update(&mut self, dt: std::time::Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.uniforms
            .update_view_proj(&self.camera, &self.projection);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("update encoder"),
            });
        let staging_buffer = self.device.create_buffer_with_data(
            bytemuck::cast_slice(&[self.uniforms]),
            wgpu::BufferUsage::COPY_SRC,
        );
        encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &self.uniform_buffer,
            0,
            std::mem::size_of::<Uniforms>() as wgpu::BufferAddress,
        );

        for instance in &mut self.instances {
            instance.rotation = Quaternion::from_angle_y(Rad(0.03)) * instance.rotation;
        }
        let instance_data = self
            .instances
            .iter()
            .map(Instance::to_raw)
            .collect::<Vec<_>>();
        let instance_buffer_size =
            instance_data.len() * std::mem::size_of::<cgmath::Matrix4<f32>>();
        let instance_buffer = self.device.create_buffer_with_data(
            bytemuck::cast_slice(&instance_data),
            wgpu::BufferUsage::COPY_SRC,
        );
        encoder.copy_buffer_to_buffer(
            &instance_buffer,
            0,
            &self.instance_buffer,
            0,
            instance_buffer_size as wgpu::BufferAddress,
        );

        self.queue.submit(&[encoder.finish()]);
    }
}

fn build_render_pipeline(
    device: &wgpu::Device,
    render_pipeline_layout: &PipelineLayout,
    vs_module: ShaderModule,
    fs_module: ShaderModule,
) -> wgpu::RenderPipeline {
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: render_pipeline_layout,
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
            // cull_mode: wgpu::CullMode::None,
            cull_mode: wgpu::CullMode::Back,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
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
            stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_read_mask: 0,
            stencil_write_mask: 0,
        }),
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[model::ModelVertex::desc()],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });
    render_pipeline
}
