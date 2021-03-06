use crate::camera::{Camera, CameraController, CursorWatcher, Projection};
use crate::drawer::render::RenderingState;
use crate::instance::{Instance, INSTANCE_DISPLACEMENT, NUM_INSTANCES_PER_ROW, NUM_ROWS};
use crate::model::{Model, ModelBatch, SimpleVertex};
use crate::texture::Texture;
use crate::widgets::fps;
use crate::{controls, drawer, event, model};
use cgmath::prelude::*;
use cgmath::{Deg, Point3, Quaternion, Vector3, Rad, Vector4};
use iced_wgpu::wgpu;
use iced_wgpu::{Backend, Renderer, Settings};
use iced_winit::winit::event_loop::EventLoop;
use iced_winit::winit::window::{Window, WindowBuilder};
use iced_winit::{conversion, program, winit, Debug, Size};
use winit::dpi::PhysicalPosition;
use winit::dpi::PhysicalSize;
use crate::drawer::model::Object;
use crate::controls::Message;

const MODELS: [&str; 2] = ["resources/penguin.obj", "resources/cube.obj"];

pub struct GUI {
    pub renderer: Renderer,
    pub program_state: program::State<controls::GUI>,
    // todo keep a list of widgets, come up with a normal design
    fps_meter: fps::Meter,
    pub cursor_position: PhysicalPosition<f64>,
    pub debug: Debug,
}

impl GUI {
    pub fn new(device: &wgpu::Device, scale_factor: f64, size: PhysicalSize<u32>) -> GUI {
        let mut renderer = iced_wgpu::Renderer::new(Backend::new(device, Settings::default()));
        let mut debug = Debug::new();
        let program_state = program::State::new(
            controls::GUI::new(),
            Size::new(size.width as f32, size.height as f32),
            conversion::cursor_position(PhysicalPosition::new(-1.0, -1.0), scale_factor),
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
    pub fn new(width: u32, height: u32) -> CameraState {
        let camera = Camera::new(Point3::new(10.0, 0.0, -25.0), Deg(90.0), Deg(0.0));
        let camera_controller = CameraController::new(4.0, 0.4);
        let projection = Projection::new(width, height, Deg(50.0), 0.1, 1000.0);
        CameraState {
            camera,
            camera_controller,
            camera_mode: false,
            projection,
            cursor_watcher: CursorWatcher::new(),
        }
    }
}

pub struct App {
    pub window: Window,
    pub resized: bool,
    pub rendering: RenderingState,
    pub camera_state: CameraState,
    objects: Vec<Object>,
}

impl App {
    pub fn run() {
        let event_loop = EventLoop::new();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let window = {
            let mut builder = WindowBuilder::new();
            builder = builder.with_title("scene-viewer");
            builder.build(&event_loop).expect("Could not build window")
        };
        let surface = unsafe { instance.create_surface(&window) };
        let rendering = RenderingState::new(
            &instance,
            surface,
            window.inner_size(),
            window.scale_factor(),
        );
        let camera_state = CameraState::new(rendering.sc_desc.width, rendering.sc_desc.height);
        let mut app = App {
            window,
            rendering,
            camera_state,
            objects: Vec::new(),
            resized: false,
        };
        app.objects = app.add_models();

        event_loop.run(move |event, _, control_flow| {
            event::processor::process_events(&mut app, &event, control_flow)
        })
    }

    fn add_models(&mut self) -> Vec<Object> {
        let mut obj_models: Vec<Model> = Vec::new();
        for model_path in MODELS.iter() {
            obj_models.push(model::Model::load(model_path).unwrap());
        }
        let mut model_batches: Vec<ModelBatch> = Vec::new();
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

            model_batches.push(ModelBatch {
                model: obj_model,
                instances,
            });
        }
        let mut objects: Vec<Object> = vec![];
        for model_batch in model_batches {
            objects.push(
                self.rendering
                    .add_model(model_batch.model, model_batch.instances),
            );
        }
        objects
    }

    pub fn resize(&mut self) {
        let new_size = self.window.inner_size();
        self.camera_state
            .projection
            .resize(new_size.width, new_size.height);
        self.rendering.sc_desc.width = new_size.width;
        self.rendering.sc_desc.height = new_size.height;
        let depth_texture = Texture::create_depth_texture(&self.rendering.sc_desc, "depth_texture");
        self.rendering.depth_texture_view = drawer::model::create_depth_view(
            &depth_texture,
            &self.rendering.device,
            &self.rendering.queue,
        );
        self.rendering.swap_chain = self
            .rendering
            .device
            .create_swap_chain(&self.rendering.surface, &self.rendering.sc_desc);
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

        for object in &mut self.objects {
            for (idx, instance) in object.instances.iter_mut().enumerate() {
                instance.rotation = Quaternion::from_angle_y(Rad(0.03)) * instance.rotation;
                self.rendering.update_instance(object.handle, idx, instance);
            }
        }

        self.rendering.gui.fps_meter.push(dt);
        self.rendering
            .gui
            .program_state
            .queue_message(controls::Message::UpdateFps(
                self.rendering.gui.fps_meter.get_average(),
            ));
    }

    pub fn process_left_click(&mut self) {
        let nd_click_coords = self.get_normalized_click_coords();
        let mut start = self.rendering.uniforms.view_proj.invert().unwrap() * nd_click_coords;
        start.x /= start.w;
        start.y /= start.w;
        start.z /= start.w;

        let ray_clip = Vector4::new(nd_click_coords.x, nd_click_coords.y, -1.0, 1.0);
        let mut ray_eye = self.camera_state.projection.calc_matrix().invert().unwrap() * ray_clip;
        ray_eye.z = -1.0;
        ray_eye.w = 0.0;
        let ray_world = (self.camera_state.camera.calc_view_matrix().invert().unwrap() * ray_eye).normalize();

        let end = start + self.camera_state.projection.zfar * ray_world;

        self.rendering.add_line(
            SimpleVertex {
                position: [
                    start.x,
                    start.y,
                    start.z
                ],
            },
            SimpleVertex {
                position: [
                    end.x,
                    end.y,
                    end.z,
                ],
            }
        );
        self.rendering.gui.program_state.queue_message(Message::DebugInfo(
            format!(
                "camera x {}, camera y {}, camera z {}",
                self.camera_state.camera.position[0], self.camera_state.camera.position[1], self.camera_state.camera.position[2]
            ),
        ));
    }

    fn get_normalized_click_coords(&self) -> Vector4<f32> {
        Vector4::new(
            (2.0 * self.rendering.gui.cursor_position.x as f32) / self.rendering.sc_desc.width as f32 - 1.0,
             1.0 - (2.0 * self.rendering.gui.cursor_position.y as f32) / self.rendering.sc_desc.height as f32,
            0.0,
            1.0
        )
    }

    pub fn render(&mut self) {
        self.rendering.render(&self.window);
    }
}
