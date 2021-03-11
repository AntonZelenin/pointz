use crate::camera::CameraState;
use crate::renderer::render::RenderingState;
use crate::model::SimpleVertex;
use crate::object::{INSTANCE_DISPLACEMENT, NUM_INSTANCES_PER_ROW, NUM_ROWS};
use crate::texture::Texture;
use crate::{renderer, editor, event, model, scene};
use cgmath::prelude::*;
use cgmath::{Deg, Quaternion, Rad, Vector3, Vector4};
use iced_wgpu::wgpu;
use iced_winit::winit::event_loop::EventLoop;
use iced_winit::winit::window::{Window, WindowBuilder};
use crate::scene::manager::Manager;
use glam::Vec3A;

const MODELS: [&str; 2] = ["resources/penguin.obj", "resources/cube.obj"];

pub struct IndexDriver {
    current_index: usize,
}

impl IndexDriver {
    pub fn new() -> IndexDriver {
        IndexDriver { current_index: 0 }
    }

    pub fn next_id(&mut self) -> usize {
        self.current_index += 1;
        self.current_index
    }
}

pub struct App {
    pub window: Window,
    pub resized: bool,
    pub rendering: RenderingState,
    pub camera_state: CameraState,
    pub scene_manager: Manager,
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
            scene_manager: Manager::new(),
        };
        app.load_models();

        event_loop.run(move |event, _, control_flow| {
            event::processor::process_events(&mut app, &event, control_flow)
        })
    }

    fn load_models(&mut self) {
        let handle1 = self.scene_manager.add_model(model::Model::load(MODELS[0]).unwrap());
        let handle2 = self.scene_manager.add_model(model::Model::load(MODELS[1]).unwrap());
        let mut i: i32 = -1;
        for model_handle in [handle1, handle2].iter() {
            i += 1;
            // let instances = (0..NUM_ROWS)
            //     .flat_map(|z| {
            //         (0..NUM_INSTANCES_PER_ROW).map(move |x| {
            //             let position = Vector3 {
            //                 x: (x * 6) as f32,
            //                 y: 0.0,
            //                 // * i * 30 just to move the second model next to the first model to showcase
            //                 z: (z * 6 + i as u32 * 30) as f32,
            //             } - INSTANCE_DISPLACEMENT;
            //             let rotation = Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0));
            //             // let rotation = if position.is_zero() {
            //             //     this is needed so an object at (0, 0, 0) won't get scaled to zero
            //             //     as Quaternions can effect scale if they're not created correctly
            //             //     Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0))
            //             // } else {
            //             //     Quaternion::from_axis_angle(position.clone().normalize(), Deg(45.0))
            //             // };
            //
            //             Instance { position, rotation }
            //         })
            //     })
            //     .collect::<Vec<_>>();
            let mut transforms = (0..NUM_ROWS)
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

                        scene::manager::Transform {
                            // position: Vec3A::new(position.x, position.y, position.z),
                            // rotation: Quat::from_axis_angle(Vec3::unit_z(), 0.0),
                            position,
                            rotation,
                            scale: Vec3A::new(1.0, 1.0, 1.0),
                        }
                    })
                })
                .collect::<Vec<_>>();
            for transform in transforms.iter_mut() {
                // todo improve, I don't need to clone it
                self.scene_manager.create_object(*model_handle, transform.clone());
            }
            let model = self.scene_manager.get_model(model_handle);
            let object_handle = self.rendering.add_instances(&model, &self.scene_manager.get_model_instances(&model_handle));
        }
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

        for (_, object) in self.scene_manager.get_objects().iter_mut() {
            object.transform.rotation = Quaternion::from_angle_y(Rad(0.03)) * object.transform.rotation;
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
