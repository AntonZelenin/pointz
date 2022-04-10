use crate::camera::CameraState;
use crate::renderer::render::RenderingState;
use crate::model::SimpleVertex;
use crate::texture::Texture;
use crate::{renderer, editor, event, model, scene};
use crate::scene::manager::{Manager, NUM_ROWS, NUM_INSTANCES_PER_ROW, INSTANCE_DISPLACEMENT};
use cgmath::prelude::*;
use cgmath::{Deg, Quaternion, Vector3, Vector4};
use iced_wgpu::wgpu;
use iced_winit::winit::event_loop::EventLoop;
use iced_winit::winit::window::{Window, WindowBuilder};
use ordered_float::OrderedFloat;
use cgmath::num_traits::Pow;

const MODELS: [&str; 3] = ["resources/penguin.obj", "resources/cube.obj", "resources/sphere.obj"];

pub struct IndexDriver {
    current_index: usize,
}

impl IndexDriver {
    pub fn new() -> IndexDriver {
        IndexDriver { current_index: 0 }
    }

    pub fn next_id(&mut self) -> usize {
        let idx = self.current_index;
        self.current_index += 1;
        idx
    }
}

pub struct App {
    pub window: Window,
    pub resized: bool,
    pub rendering: RenderingState,
    pub camera_state: CameraState,
    pub scene_manager: Manager,
    pub model_loader: model::Loader,
}

impl App {
    pub fn run() {
        let event_loop = EventLoop::new();
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
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
        let camera_state = CameraState::new(rendering.surface_config.width, rendering.surface_config.height);
        let mut app = App {
            window,
            rendering,
            camera_state,
            resized: false,
            scene_manager: Manager::new(),
            model_loader: model::Loader::new(),
        };
        app.add_objects();

        event_loop.run(move |event, _, control_flow| {
            event::processor::process_events(&mut app, &event, control_flow)
        })
    }

    fn add_objects(&mut self) {
        self.scene_manager.add_model(self.model_loader.load(MODELS[0]).unwrap());
        self.scene_manager.add_model(self.model_loader.load(MODELS[1]).unwrap());
        let bounding_model_id = self.scene_manager.add_model(self.model_loader.load_primitive(MODELS[2]).unwrap());
        let mut i: i32 = -1;
        let bounding_sphere = self.scene_manager.get_model(bounding_model_id);
        self.rendering.init_bounding_sphere(&bounding_sphere);
        for model_id in self.scene_manager.get_model_ids().iter() {
            if *model_id == bounding_model_id {
                continue;
            }
            i += 1;

            let new_instances = self.create_instances(i, *model_id);
            let new_bounding_spheres = self.create_bounding_sphere_instances(*model_id, bounding_model_id);

            let model = self.scene_manager.get_model(*model_id);
            self.rendering.init_model(&model);
            self.rendering.add_instances(&model, &self.scene_manager.get_objects_by_ids(&new_instances));
            self.rendering.add_bounding_sphere_instances(bounding_model_id, &self.scene_manager.get_objects_by_ids(&new_bounding_spheres));
        }
    }

    fn create_instances(&mut self, i: i32, model_id: usize) ->Vec<usize> {
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
                        position,
                        rotation,
                        scale: Vector3::new(1.0, 1.0, 1.0),
                    }
                })
            })
            .collect::<Vec<_>>();
        let mut ids: Vec<usize> = vec![];
        for transform in transforms.iter_mut() {
            ids.push(self.scene_manager.create_object(model_id.clone(), transform.clone()));
        }
        ids
    }

    fn create_bounding_sphere_instances(&mut self, model_id: usize, bounding_model_id: usize) ->Vec<usize> {
        let model = self.scene_manager.get_model(model_id);
        let radius = calc_bounding_sphere_radius(model);

        let mut transforms = vec![];
        for object in self.scene_manager.get_model_instances(model_id) {
            let mut transform = object.transform.clone();
            transform.scale *= radius;
            transforms.push(transform);
        }
        let mut ids: Vec<usize> = vec![];
        for transform in transforms {
            ids.push(self.scene_manager.create_object(bounding_model_id, transform));
        }
        ids
    }

    pub fn resize(&mut self) {
        let new_size = self.window.inner_size();
        self.camera_state
            .projection
            .resize(new_size.width, new_size.height);
        self.rendering.surface_config.width = new_size.width;
        self.rendering.surface_config.height = new_size.height;
        let depth_texture = Texture::create_depth_texture(&self.rendering.surface_config, "depth_texture");
        self.rendering.depth_texture_view = renderer::model::create_depth_view(
            &depth_texture,
            &self.rendering.device,
            &self.rendering.queue,
        );
        self.rendering.surface.configure(&self.rendering.device, &self.rendering.surface_config);
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

        // todo disabled rotation because models and bounding spheres are all in the schene_manager and I try to rotate all object
        // todo and in fact models and spheres are in different renderers
        // for (_, object) in self.scene_manager.get_objects().iter_mut() {
        //     object.transform.rotation = Quaternion::from_angle_y(Rad(0.03)) * object.transform.rotation;
        //     self.rendering.update_object(object);
        // }

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
                / self.rendering.surface_config.width as f32
                - 1.0,
            1.0 - (2.0 * self.rendering.gui.cursor_position.y as f32)
                / self.rendering.surface_config.height as f32,
            0.0,
            1.0,
        )
    }

    pub fn render(&mut self) {
        self.rendering.render(&self.window);
    }
}

// todo move
fn calc_bounding_sphere_radius(model: &model::Model) -> f32 {
    let mut lengths: Vec<OrderedFloat<f32>> = vec![];
    for mesh in model.meshes.iter() {
        lengths = mesh.vertices.iter().map(|vertex| {
            // todo move to a math lib? or it already exists?
            // we measure the distance between the model space 0,0,0 and a vertex, so vertex vector will always be the same as it's coords
            let length: f32 = vertex.position.x.pow(2) + vertex.position.y.pow(2) + vertex.position.z.pow(2);
            OrderedFloat(length.sqrt())
        }).collect();
    }
    let max = lengths.iter().max().unwrap().into_inner();
    max
}
