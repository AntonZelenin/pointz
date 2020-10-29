use crate::buffer::Uniforms;
use crate::camera::{Camera, CameraController, Projection};
use crate::controls::{Message, GUI};
use crate::instance::{Instance, INSTANCE_DISPLACEMENT, NUM_INSTANCES_PER_ROW, NUM_ROWS};
use crate::lighting::{DrawLight, Light};
use crate::model;
use crate::model::{DrawModel, Model, ModelData, Vertex};
use crate::texture::Texture;
use crate::widgets::fps;
use cgmath::prelude::*;
use cgmath::{Deg, Point3, Quaternion, Rad, Vector2, Vector3, Vector4};
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::util::DeviceExt;
use iced_wgpu::wgpu::{PipelineLayout, ShaderModule};
use iced_wgpu::{Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Debug, Size};
use std::iter;
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::{dpi::PhysicalPosition, event::ModifiersState, window::Window};

const KEEP_CURSOR_POS_FOR_NUM_FRAMES: usize = 3;

const MODELS: [&str; 2] = ["resources/penguin.obj", "resources/cube.obj"];

pub struct State {
    viewport: Viewport,
    surface: wgpu::Surface,
    window: Window,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    render_pipeline: wgpu::RenderPipeline,
    queue: wgpu::Queue,
    device: wgpu::Device,
    renderer: Renderer,
    program_state: program::State<GUI>,
    depth_texture: Texture,
    model_data: Vec<ModelData>,
    uniforms: Uniforms,
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
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,

    fps_meter: fps::Meter,

    debug_render_pipeline: wgpu::RenderPipeline,
    vec_start: Vector3<f32>,
    vec_end: Vector3<f32>,
}

impl State {
    pub fn new(window: winit::window::Window) -> State {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let (mut device, queue) = futures::executor::block_on(async {
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

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera, &projection);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            label: Some("uniform buffer"),
        });
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

        let mut obj_models: Vec<Model> = Vec::new();
        for model_path in MODELS.iter() {
            obj_models.push(
                model::Model::load(&device, &queue, &texture_bind_group_layout, model_path)
                    .unwrap(),
            );
        }

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
            let vs_module = device.create_shader_module(wgpu::include_spirv!("shader/spv/shader.vert.spv"));
            let fs_module = device.create_shader_module(wgpu::include_spirv!("shader/spv/shader.frag.spv"));
            build_render_pipeline(&device, &render_pipeline_layout, vs_module, fs_module)
        };

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("light pipeline"),
                bind_group_layouts: &[&uniform_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });
            let vs_module = device.create_shader_module(wgpu::include_spirv!("shader/spv/light.vert.spv"));
            let fs_module = device.create_shader_module(wgpu::include_spirv!("shader/spv/light.frag.spv"));
            build_render_pipeline(&device, &layout, vs_module, fs_module)
        };
        let mut models: Vec<ModelData> = Vec::new();
        let mut i: i32 = -1;
        for obj_model in obj_models {
            i += 1;
            let instances = (0..NUM_ROWS)
                .flat_map(|z| {
                    (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                        let position = Vector3 {
                            x: (x * 6) as f32,
                            y: 0.0,
                            // * i * 30 just to move the second model next to the first model to showcase
                            z: (z * 6 + i as u32 * 30) as f32,
                        } - INSTANCE_DISPLACEMENT;
                        let rotation = Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0));
                        // let rotation = if position.is_zero() {
                        //     this is needed so an object at (0, 0, 0) won't get scaled to zero
                        //     as Quaternions can effect scale if they're not created correctly
                        //     Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0))
                        // } else {
                        //     Quaternion::from_axis_angle(position.clone().normalize(), Deg(45.0))
                        // };

                        Instance { position, rotation }
                    })
                })
                .collect::<Vec<_>>();

            let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
            let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
                label: Some("instance buffer"),
            });
            let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &uniform_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(instance_buffer.slice(..)),
                    },
                ],
                label: Some("uniform_bind_group"),
            });

            models.push(ModelData {
                model: obj_model,
                instances,
                instance_buffer,
                uniform_bind_group,
            });
        }

        let debug_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("debug pipeline"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
            let vs_module = device.create_shader_module(wgpu::include_spirv!("shader/spv/debug.vert.spv"));
            let fs_module = device.create_shader_module(wgpu::include_spirv!("shader/spv/debug.frag.spv"));
            build_render_pipeline(&device, &layout, vs_module, fs_module)
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
            model_data: models,
            uniforms,
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
            light_bind_group,
            light_render_pipeline,
            fps_meter: fps::Meter::new(),
            debug_render_pipeline,
            vec_start: Vector3::new(0.0, 0.0, 0.0),
            vec_end: Vector3::new(0.0, 0.0, 0.0),
        }
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
                    WindowEvent::MouseInput {
                        button: MouseButton::Left,
                        state,
                        ..
                    } => {
                        if *state == ElementState::Pressed {
                            self.process_left_click();
                        }
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
                self.render();
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

    fn render(&mut self) {
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

            // render_pass.set_pipeline(&self.light_render_pipeline);
            // render_pass.draw_light_model(
            //     &self.model_data[0].model,
            //     &self.model_data[0].uniform_bind_group,
            //     &self.light_bind_group,
            // );

            render_pass.set_pipeline(&self.render_pipeline);
            for model_data in &mut self.model_data {
                render_pass.draw_model_instanced(
                    &model_data.model,
                    0..model_data.instances.len() as u32,
                    &model_data.uniform_bind_group,
                    &self.light_bind_group,
                );
            }
        }
        let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
        let mouse_interaction = self.renderer.backend_mut().draw(
            &mut self.device,
            &mut staging_belt,
            &mut encoder,
            &frame.view,
            &self.viewport,
            self.program_state.primitive(),
            &self.debug.overlay(),
        );
        self.window
            .set_cursor_icon(iced_winit::conversion::mouse_interaction(mouse_interaction));
        staging_belt.finish();
        self.queue.submit(iter::once(encoder.finish()));
    }

    fn update(&mut self, dt: std::time::Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);

        self.uniforms
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        for model_data in &mut self.model_data {
            for instance in &mut model_data.instances {
                instance.rotation = Quaternion::from_angle_y(Rad(0.03)) * instance.rotation;
            }
            let instance_data = model_data
                .instances
                .iter()
                .map(Instance::to_raw)
                .collect::<Vec<_>>();
            self.queue.write_buffer(
                &model_data.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_data),
            );
        }

        self.fps_meter.push(dt);
        self.program_state
            .queue_message(Message::UpdateFps(self.fps_meter.get_average()));
    }

    fn process_left_click(&mut self) {
        let click_coords = self.get_normalized_opengl_coords();
        let clip_coords = Vector4::new(click_coords.x, click_coords.y, -1.0, 1.0);
        // y is deviated by 2 for some reason
        let click_world_coords = self.camera.calc_matrix().invert().unwrap() * clip_coords;
        let mut eye_coords = self.projection.calc_matrix().invert().unwrap() * clip_coords;
        eye_coords = Vector4::new(eye_coords.x, eye_coords.y, -1.0, 0.0);
        let ray_world = (self.camera.calc_matrix().invert().unwrap() * eye_coords).normalize();
        self.vec_start = Vector3::new(click_world_coords.x, click_world_coords.y, click_world_coords.z);
        self.vec_end = Vector3::new(ray_world.x, ray_world.y, ray_world.z) * self.projection.zfar;
        // self.program_state.queue_message(Message::DebugInfo(
        //     format!(
        //         "x {}, y {}, z {}\n",
        //         click_world_coords.x, click_world_coords.y, click_world_coords.z
        //     ) + &format!(
        //         "x {}, y {}, z {}",
        //         self.camera.position.x, self.camera.position.y, self.camera.position.z
        //     ),
        // ));
    }

    fn get_normalized_opengl_coords(&self) -> Vector2<f32> {
        // convert mouse position to opengl coords
        Vector2::new(
            (2.0 * self.cursor_position.x as f32) / self.sc_desc.width as f32 - 1.0,
            -(2.0 * self.cursor_position.y as f32) / self.sc_desc.height as f32 - 1.0,
        )
    }
}

fn build_render_pipeline(
    device: &wgpu::Device,
    render_pipeline_layout: &PipelineLayout,
    vs_module: ShaderModule,
    fs_module: ShaderModule,
) -> wgpu::RenderPipeline {
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
            vertex_buffers: &[model::ModelVertex::desc()],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });
    render_pipeline
}
