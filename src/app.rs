use crate::camera::CameraState;
use crate::renderer::render::RenderingState;
use crate::model::{Model, SimpleVertex};
use crate::object::{Instance, Object, INSTANCE_DISPLACEMENT, NUM_INSTANCES_PER_ROW, NUM_ROWS};
use crate::texture::Texture;
use crate::widgets::fps;
use crate::{renderer, editor, event, model};
use cgmath::prelude::*;
use cgmath::{Deg, Quaternion, Rad, Vector3, Vector4};
use iced_wgpu::wgpu;
use iced_wgpu::{Backend, Renderer, Settings};
use iced_winit::winit::event_loop::EventLoop;
use iced_winit::winit::window::{Window, WindowBuilder};
use iced_winit::{conversion, program, winit, Debug, Size};
use legion;
use winit::dpi::PhysicalPosition;
use winit::dpi::PhysicalSize;

const MODELS: [&str; 2] = ["resources/penguin.obj", "resources/cube.obj"];

pub struct GUI {
    pub renderer: Renderer,
    pub program_state: program::State<editor::GUI>,
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
            editor::GUI::new(),
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

pub struct App {
    pub window: Window,
    pub resized: bool,
    pub rendering: RenderingState,
    pub camera_state: CameraState,
    pub world: World,
}

pub struct World {
    objects: Vec<Object>,
    entities: legion::World,
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
        // world.push((position,));
        // if let Some(mut entry) = world.entry(object.components[0]) {
        //     let position = entry.into_component::<Position>();
        // }
        let mut app = App {
            window,
            rendering,
            camera_state,
            resized: false,
            world: World {
                objects: Vec::new(),
                entities: legion::World::default(),
            },
        };
        app.load_models();

        event_loop.run(move |event, _, control_flow| {
            event::processor::process_events(&mut app, &event, control_flow)
        })
    }

    fn load_models(&mut self) {
        let mut obj_models: Vec<Model> = Vec::new();
        for model_path in MODELS.iter() {
            obj_models.push(model::Model::load(model_path).unwrap());
        }
        let mut objects: Vec<Object> = vec![];
        let mut i: i32 = -1;
        for model in obj_models {
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
            let object_handle = self.rendering.add_model(&model, &instances);
            for (idx, instance) in instances.iter().enumerate() {
                let object = Object {
                    handle: object_handle,
                    instance_index: idx,
                    position: instance.position,
                    rotation: instance.rotation,
                    components: vec![],
                };
                // object
                //     .components
                //     .push(self.world.entities.push((physics::BoundingSphere {
                //         center: Vec3::new(
                //             instance.position.x,
                //             instance.position.y,
                //             instance.position.z,
                //         ),
                //         radius: calc_bounding_sphere_radius(&model),
                //     },)));
                objects.push(object);
            }
        }
        self.world.objects = objects;
    }

    pub fn resize(&mut self) {
        let new_size = self.window.inner_size();
        self.camera_state
            .projection
            .resize(new_size.width, new_size.height);
        self.rendering.sc_desc.width = new_size.width;
        self.rendering.sc_desc.height = new_size.height;
        let depth_texture = Texture::create_depth_texture(&self.rendering.sc_desc, "depth_texture");
        self.rendering.depth_texture_view = renderer::model::create_depth_view(
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

        for object in &mut self.world.objects.iter_mut() {
            object.rotation = Quaternion::from_angle_y(Rad(0.03)) * object.rotation;
            self.rendering.update_object(object);
        }

        self.rendering.gui.fps_meter.push(dt);
        self.rendering
            .gui
            .program_state
            .queue_message(editor::Message::UpdateFps(
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
                position: [start.x, start.y, start.z],
            },
            SimpleVertex {
                position: [end.x, end.y, end.z],
            },
        );
        self.rendering
            .gui
            .program_state
            .queue_message(editor::Message::DebugInfo(format!(
                "camera x {}, camera y {}, camera z {}",
                self.camera_state.camera.position[0],
                self.camera_state.camera.position[1],
                self.camera_state.camera.position[2]
            )));
    }

    fn get_normalized_click_coords(&self) -> Vector4<f32> {
        Vector4::new(
            (2.0 * self.rendering.gui.cursor_position.x as f32)
                / self.rendering.sc_desc.width as f32
                - 1.0,
            1.0 - (2.0 * self.rendering.gui.cursor_position.y as f32)
                / self.rendering.sc_desc.height as f32,
            0.0,
            1.0,
        )
    }

    pub fn render(&mut self) {
        self.rendering.render(&self.window);
    }
}
