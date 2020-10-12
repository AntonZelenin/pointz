use std::iter;
use crate::buffer::Uniforms;
use crate::camera::{Camera, CameraController, Projection};
use crate::controls::GUI;
use crate::instance::{Instance, INSTANCE_DISPLACEMENT, NUM_INSTANCES_PER_ROW, NUM_ROWS};
use crate::{model, texture};
use crate::model::{Vertex, DrawModel, Model, Material};
use crate::texture::Texture;
use cgmath::prelude::*;
use cgmath::{Deg, Point3, Quaternion, Rad, Vector3};
// #[macro_use]
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
use iced_wgpu::wgpu::util::DeviceExt;

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
    debug_material: Material,
}

impl State {
    pub fn new(window: winit::window::Window) -> State {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let (mut device, queue) = futures::executor::block_on(async {
            let adapter = instance.request_adapter(
                &wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::Default,
                    compatible_surface: Some(&surface),
                },
            )
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
        // let instance_buffer_size = instance_data.len() * std::mem::size_of::<Matrix4<f32>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsage::STORAGE,
            label: Some("instance buffer"),
        });
        println!("4 {:?}", instance_buffer);
        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera, &projection);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            label: Some("uniform buffer"),
        });
        println!("5 {:?}", uniform_buffer);
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            // todo bind size everywhere
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

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            // component_type: wgpu::TextureComponentType::Uint,
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
        println!("6 {:?}", light_buffer);
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

        let obj_model = model::Model::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            "resources/cube.obj",
        ).unwrap();

        // queue.submit(command_buffers);

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
            let vs_module =
                device.create_shader_module(wgpu::include_spirv!("shader/vert.spv"));
            let fs_module =
                device.create_shader_module(wgpu::include_spirv!("shader/frag.spv"));
            build_render_pipeline(&device, &render_pipeline_layout, vs_module, fs_module)
        };

        let
            light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("light pipeline"),
                bind_group_layouts: &[
                    &uniform_bind_group_layout,
                    &light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
            let vs_module =
                device.create_shader_module(wgpu::include_spirv!("shader/light_vert.spv"));
            let fs_module =
                device.create_shader_module(wgpu::include_spirv!("shader/light_frag.spv"));
            build_render_pipeline(
                &device,
                &layout,
                vs_module,
                fs_module,
            )
        };

        let debug_material = {
            let diffuse_bytes = include_bytes!("../resources/cobble-diffuse.png");
            let normal_bytes = include_bytes!("../resources/cobble-normal.png");

            // let mut command_buffers = vec![];
            let diffuse_texture = texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "res/alt-diffuse.png", false).unwrap();
            // command_buffers.push(cmds);
            let normal_texture = texture::Texture::from_bytes(&device, &queue, normal_bytes, "res/alt-normal.png", true).unwrap();
            // command_buffers.push(cmds);
            // queue.submit(command_buffers);

            model::Material::new(&device, "alt-material", diffuse_texture, normal_texture, &texture_bind_group_layout)
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
            debug_material,
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
        render_pass.draw_model_instanced_with_material(
            &self.obj_model,
            &self.debug_material,
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
                self.render2();
                // хехехе, я закомментил вот этот кусок и стало работать)
                // if self.resized {
                //     self.resize(self.window.inner_size());
                //     self.resized = false;
                // }
                // let frame = self.swap_chain.get_current_frame().expect("Next frame");
                // let mut encoder = self
                //     .device
                //     .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                // let program = self.program_state.program();
                // {
                //     let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                //         color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                //             attachment: &frame.output.view,
                //             resolve_target: None,
                //             ops: {
                //                 let [r, g, b, a] = program.background_color().into_linear();
                //                 wgpu::Operations {
                //                     load: wgpu::LoadOp::Clear(wgpu::Color {
                //                         r: r as f64,
                //                         g: g as f64,
                //                         b: b as f64,
                //                         a: a as f64,
                //                     }),
                //                     store: true,
                //                 }
                //             },
                //         }],
                //         depth_stencil_attachment: Some(
                //             wgpu::RenderPassDepthStencilAttachmentDescriptor {
                //                 attachment: &self.depth_texture.view,
                //                 depth_ops: Some(wgpu::Operations {
                //                     load: wgpu::LoadOp::Clear(1.0),
                //                     store: true,
                //                 }),
                //                 stencil_ops: Some(wgpu::Operations {
                //                     load: wgpu::LoadOp::Clear(0),
                //                     store: true,
                //                 }),
                //             },
                //         ),
                //     });
                //     self.draw(&mut render_pass);
                // }
                // // todo what is this?
                // let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
                // let mouse_interaction = self.renderer.backend_mut().draw(
                //     &mut self.device,
                //     &mut staging_belt,
                //     &mut encoder,
                //     &frame.output.view,
                //     &self.viewport,
                //     self.program_state.primitive(),
                //     &self.debug.overlay(),
                // );
                // self.queue.submit(iter::once(encoder.finish()));
                // self.window
                //     .set_cursor_icon(iced_winit::conversion::mouse_interaction(mouse_interaction));
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

    fn render2(&mut self) {
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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.light_render_pipeline);
            render_pass.draw_light_model(
                &self.obj_model,
                &self.uniform_bind_group,
                &self.light_bind_group,
            );

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw_model_instanced(
                &self.obj_model,
                0..self.instances.len() as u32,
                &self.uniform_bind_group,
                &self.light_bind_group,
            );
        }
        self.queue.submit(iter::once(encoder.finish()));
    }

    fn update(&mut self, dt: std::time::Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.uniforms
            .update_view_proj(&self.camera, &self.projection);
        // let mut encoder = self
        //     .device
        //     .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        //         label: Some("update encoder"),
        //     });
        // let staging_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     contents: bytemuck::cast_slice(&[self.uniforms]),
        //     usage: wgpu::BufferUsage::COPY_SRC,
        //     label: Some("staging buffer"),
        // });
        // encoder.copy_buffer_to_buffer(
        //     &staging_buffer,
        //     0,
        //     &self.uniform_buffer,
        //     0,
        //     std::mem::size_of::<Uniforms>() as wgpu::BufferAddress,
        // );
        self.queue.write_buffer(&self.uniform_buffer, 0, &bytemuck::cast_slice(&[self.uniforms]));

        for instance in &mut self.instances {
            instance.rotation = Quaternion::from_angle_y(Rad(0.03)) * instance.rotation;
        }
        // let instance_data = self
        //     .instances
        //     .iter()
        //     .map(Instance::to_raw)
        //     .collect::<Vec<_>>();
        // let instance_buffer_size =
        //     instance_data.len() * std::mem::size_of::<cgmath::Matrix4<f32>>();
        // let instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     contents: bytemuck::cast_slice(&instance_data),
        //     // usage: wgpu::BufferUsage::COPY_SRC,
        //     usage: wgpu::BufferUsage::STORAGE,
        //     label: Some("instance buffer"),
        // });
        // encoder.copy_buffer_to_buffer(
        //     &instance_buffer,
        //     0,
        //     &self.instance_buffer,
        //     0,
        //     instance_buffer_size as wgpu::BufferAddress,
        // );

        // self.queue.submit(Some(encoder.finish()));
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
            // cull_mode: wgpu::CullMode::None,
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
