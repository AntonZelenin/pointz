use crate::buffer::Uniforms;
use crate::camera::{Camera, CameraController, CursorWatcher, Projection};
use crate::controls;
use crate::instance::{Instance, INSTANCE_DISPLACEMENT, NUM_INSTANCES_PER_ROW, NUM_ROWS};
use crate::lighting::Light;
use crate::model;
use crate::model::{DrawModel, Model, ModelData, SimpleVertex, Vertex};
use crate::texture::Texture;
use crate::widgets::fps;
use cgmath::prelude::*;
use cgmath::{Deg, Point3, Quaternion, Rad, Vector2, Vector3, Vector4};
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::util::DeviceExt;
use iced_wgpu::wgpu::{PipelineLayout, ShaderModule};
use iced_wgpu::{Backend, Renderer, Settings, Viewport};
use iced_winit::winit::event_loop::EventLoop;
use iced_winit::winit::window;
use iced_winit::{conversion, futures, program, winit, Debug, Size};
use std::iter;
use std::time::Instant;
use winit::dpi::PhysicalPosition;
use winit::dpi::PhysicalSize;

const MODELS: [&str; 2] = ["resources/penguin.obj", "resources/cube.obj"];
const INDICES: &[u32] = &[0, 1];

pub struct GUI {
    pub renderer: Renderer,
    pub program_state: program::State<controls::GUI>,
    // todo keep a list of widgets, come up with a normal design
    fps_meter: fps::Meter,
    pub cursor_position: PhysicalPosition<f64>,
    pub debug: Debug,
}

impl GUI {
    pub fn new(device: &mut wgpu::Device, viewport: &Viewport) -> GUI {
        let mut renderer = iced_wgpu::Renderer::new(Backend::new(device, Settings::default()));
        let mut debug = Debug::new();
        let program_state = program::State::new(
            controls::GUI::new(),
            viewport.logical_size(),
            conversion::cursor_position(PhysicalPosition::new(-1.0, -1.0), viewport.scale_factor()),
            &mut renderer,
            &mut debug,
        );
        GUI {
            renderer,
            program_state,
            fps_meter: fps::Meter::new(),
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            debug,
        }
    }
}

pub struct CameraState {
    camera: Camera,
    pub camera_controller: CameraController,
    pub camera_mode: bool,
    projection: Projection,
    pub cursor_watcher: CursorWatcher,
}

impl CameraState {
    pub fn new(sc_desc: &wgpu::SwapChainDescriptor) -> CameraState {
        let camera = Camera::new(Point3::new(-30.0, 25.0, 25.0), Deg(0.0), Deg(-40.0));
        let camera_controller = CameraController::new(4.0, 0.4);
        let projection = Projection::new(sc_desc.width, sc_desc.height, Deg(50.0), 0.1, 1000.0);
        CameraState {
            camera,
            camera_controller,
            camera_mode: false,
            projection,
            cursor_watcher: CursorWatcher::new(),
        }
    }
}

pub struct Window {
    pub viewport: Viewport,
    pub surface: wgpu::Surface,
    pub window: winit::window::Window,
    pub resized: bool,
}

impl Window {
    pub fn new(instance: &wgpu::Instance, event_loop: &EventLoop<()>) -> Window {
        let window = window::Window::new(&event_loop).unwrap();
        let viewport = Viewport::with_physical_size(
            Size::new(window.inner_size().width, window.inner_size().height),
            window.scale_factor(),
        );
        let surface = unsafe { instance.create_surface(&window) };

        let window = Window {
            viewport,
            surface,
            window,
            resized: false,
        };
        window
    }
}

pub struct Rendering {
    pub sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,

    pub queue: wgpu::Queue,
    pub device: wgpu::Device,

    render_pipeline: wgpu::RenderPipeline,
    debug_render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,

    uniforms: Uniforms,
    pub uniform_buffer: wgpu::Buffer,
    debug_buff: wgpu::Buffer,
    index_buff: wgpu::Buffer,

    light_bind_group: wgpu::BindGroup,
    debug_uniform_bind_group: wgpu::BindGroup,

    // todo move
    depth_texture: Texture,
    pub last_render_time: Instant,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_bind_group_layout: wgpu::BindGroupLayout,
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
        let debug_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("debug pipeline"),
                bind_group_layouts: &[&debug_uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
            let vs_module =
                device.create_shader_module(wgpu::include_spirv!("shader/spv/debug.vert.spv"));
            let fs_module =
                device.create_shader_module(wgpu::include_spirv!("shader/spv/debug.frag.spv"));
            build_render_pipeline(&device, &layout, vs_module, fs_module)
        };

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
                device.create_shader_module(wgpu::include_spirv!("shader/spv/light.vert.spv"));
            let fs_module =
                device.create_shader_module(wgpu::include_spirv!("shader/spv/light.frag.spv"));
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
        let index_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Debug Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsage::INDEX,
        });
        let debug_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &debug_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
            }],
            label: Some("debug_uniform_bind_group"),
        });

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

        Rendering {
            swap_chain,
            sc_desc,
            render_pipeline,
            queue,
            device,
            depth_texture,
            last_render_time: std::time::Instant::now(),
            debug_render_pipeline,
            light_render_pipeline,
            uniforms,
            uniform_buffer,
            light_bind_group,
            debug_buff,
            index_buff,
            debug_uniform_bind_group,
            texture_bind_group_layout,
            uniform_bind_group_layout,
        }
    }
}

pub struct Scene {
    pub model_data: Vec<ModelData>,
}

impl Scene {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
        uniform_buffer: &wgpu::Buffer,
    ) -> Scene {
        let mut obj_models: Vec<Model> = Vec::new();
        for model_path in MODELS.iter() {
            obj_models.push(
                model::Model::load(device, queue, texture_bind_group_layout, model_path).unwrap(),
            );
        }
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
                layout: uniform_bind_group_layout,
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
        Scene { model_data: models }
    }
}

pub struct App {
    pub window: Window,
    pub rendering: Rendering,
    pub gui: GUI,
    pub camera_state: CameraState,
    pub scene: Scene,
}

impl App {
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.camera_state
            .projection
            .resize(new_size.width, new_size.height);
        // todo event
        self.rendering.sc_desc.width = new_size.width;
        self.rendering.sc_desc.height = new_size.height;
        self.rendering.depth_texture = Texture::create_depth_texture(
            &self.rendering.device,
            &self.rendering.sc_desc,
            "depth_texture",
        );
        self.rendering.swap_chain = self
            .rendering
            .device
            .create_swap_chain(&self.window.surface, &self.rendering.sc_desc);
    }

    pub fn render(&mut self) {
        let frame = self
            .rendering
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting texture")
            .output;

        let mut encoder =
            self.rendering
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
                    attachment: &self.rendering.depth_texture.view,
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

            render_pass.set_pipeline(&self.rendering.render_pipeline);
            for model_data in &mut self.scene.model_data {
                render_pass.draw_model_instanced(
                    &model_data.model,
                    0..model_data.instances.len() as u32,
                    &model_data.uniform_bind_group,
                    &self.rendering.light_bind_group,
                );
            }

            render_pass.set_pipeline(&self.rendering.debug_render_pipeline);
            render_pass.set_vertex_buffer(0, self.rendering.debug_buff.slice(..));
            render_pass.set_index_buffer(self.rendering.index_buff.slice(..));
            render_pass.set_bind_group(0, &self.rendering.debug_uniform_bind_group, &[]);
            render_pass.draw_indexed(0..2, 0, 0..1);
        }

        let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
        let mouse_interaction = self.gui.renderer.backend_mut().draw(
            &mut self.rendering.device,
            &mut staging_belt,
            &mut encoder,
            &frame.view,
            &self.window.viewport,
            self.gui.program_state.primitive(),
            &self.gui.debug.overlay(),
        );
        self.window
            .window
            .set_cursor_icon(iced_winit::conversion::mouse_interaction(mouse_interaction));
        staging_belt.finish();
        self.rendering.queue.submit(iter::once(encoder.finish()));
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.camera_state
            .camera_controller
            .update_camera(&mut self.camera_state.camera, dt);

        self.rendering
            .uniforms
            .update_view_proj(&self.camera_state.camera, &self.camera_state.projection);
        self.rendering.queue.write_buffer(
            &self.rendering.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.rendering.uniforms]),
        );

        for model_data in &mut self.scene.model_data {
            for instance in &mut model_data.instances {
                instance.rotation = Quaternion::from_angle_y(Rad(0.03)) * instance.rotation;
            }
            let instance_data = model_data
                .instances
                .iter()
                .map(Instance::to_raw)
                .collect::<Vec<_>>();
            self.rendering.queue.write_buffer(
                &model_data.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_data),
            );
        }

        self.gui.fps_meter.push(dt);
        self.gui
            .program_state
            .queue_message(controls::Message::UpdateFps(
                self.gui.fps_meter.get_average(),
            ));
    }

    pub fn process_left_click(&mut self) {
        let click_coords = self.get_normalized_opengl_coords();
        let clip_coords = Vector4::new(click_coords.x, click_coords.y, -1.0, 1.0);
        // y is deviated by 2 for some reason
        let click_world_coords =
            self.camera_state.camera.calc_matrix().invert().unwrap() * clip_coords;
        let mut eye_coords =
            self.camera_state.projection.calc_matrix().invert().unwrap() * clip_coords;
        eye_coords = Vector4::new(eye_coords.x, eye_coords.y, -1.0, 0.0);
        let ray_world =
            (self.camera_state.camera.calc_matrix().invert().unwrap() * eye_coords).normalize();
        let vec_start = SimpleVertex {
            position: [
                click_world_coords.x,
                click_world_coords.y,
                click_world_coords.z,
            ],
        };
        let vec_end = SimpleVertex {
            position: [
                ray_world.x * self.camera_state.projection.zfar,
                ray_world.y * self.camera_state.projection.zfar,
                ray_world.z * self.camera_state.projection.zfar,
            ],
        };
        self.rendering.debug_buff =
            self.rendering
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&[vec_start, vec_end]),
                    usage: wgpu::BufferUsage::VERTEX,
                });
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
            (2.0 * self.gui.cursor_position.x as f32) / self.rendering.sc_desc.width as f32 - 1.0,
            -(2.0 * self.gui.cursor_position.y as f32) / self.rendering.sc_desc.height as f32 - 1.0,
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
