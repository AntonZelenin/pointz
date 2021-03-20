use crate::renderer::render;
use crate::{model};
use crate::model::{Vertex, SimpleVertex, Model};
use crate::app::IndexDriver;
use crate::scene::manager::Object;
use crate::renderer::render::{InternalModel, InternalMesh};
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::util::DeviceExt;
use iced_wgpu::wgpu::RenderPass;
use std::ops::Range;
use std::collections::HashMap;
use ordered_float::OrderedFloat;
use cgmath::num_traits::Pow;

#[repr(C)]
#[derive(Copy, Clone)]
struct Transform {
    // it has padding!
    // position: Vec3A,
    center: [f32; 3],
    radius: f32,
}

unsafe impl bytemuck::Pod for Transform {}
unsafe impl bytemuck::Zeroable for Transform {}

pub struct BoundingSpheresDrawer2 {
    index_driver: IndexDriver,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer_registry: HashMap<usize, wgpu::Buffer>,
    index_buffer_registry: HashMap<usize, wgpu::Buffer>,
    internal_model: InternalModel,
    uniform_bind_group_registry: HashMap<usize, wgpu::BindGroup>,
    instance_buffer_registry: HashMap<usize, wgpu::Buffer>,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl BoundingSpheresDrawer2 {
    pub fn new(device: &wgpu::Device, model: &Model) -> BoundingSpheresDrawer2 {
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: Some("bounding sphere uniform bind group layout"),
            });
        let render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("bounding sphere pipeline"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
            let vs_module =
                device.create_shader_module(&wgpu::include_spirv!("../shader/spv/bounding_sphere.vert.spv"));
            let fs_module =
                device.create_shader_module(&wgpu::include_spirv!("../shader/spv/bounding_sphere.frag.spv"));
            render::build_render_pipeline(device, &layout, vs_module, fs_module, SimpleVertex::desc(), wgpu::PrimitiveTopology::TriangleList)
        };

        let mut index_driver = IndexDriver::new();
        let mut internal_meshes: Vec<InternalMesh> = vec![];
        let mut index_buffer_registry = HashMap::new();
        let mut vertex_buffer_registry = HashMap::new();
        for mesh in model.meshes.iter() {
            let mesh_id = index_driver.next_id();
            index_buffer_registry
                .insert(mesh_id, <BoundingSpheresDrawer2>::create_mesh_index_buffer(&mesh, device));
            vertex_buffer_registry
                .insert(mesh_id, <BoundingSpheresDrawer2>::create_vertex_buffer(mesh, device));
            internal_meshes.push(InternalMesh {
                count: mesh.indices.len(),
                id: mesh_id,
                material_id: 0,
            });
        }
        let internal_model = InternalModel {
            id: model.id,
            num_of_instances: 0,
            internal_meshes,
        };

        BoundingSpheresDrawer2 {
            index_driver,
            render_pipeline,
            uniform_bind_group_registry: HashMap::new(),
            vertex_buffer_registry,
            index_buffer_registry,
            instance_buffer_registry: HashMap::new(),
            uniform_bind_group_layout,
            internal_model,
        }
    }

    pub fn add(&mut self, model: &model::Model, sphere_instances: &Vec<&Object>, device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) {
        let mut transforms: Vec<Transform> = vec![];
        for instance in sphere_instances.iter() {
            transforms.push(Transform {
                center: [instance.transform.position.x, instance.transform.position.y, instance.transform.position.z],
                radius: calc_bounding_sphere_radius(&model),
            });
        }
        // todo should I drop existing buffer?
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&transforms),
            usage: wgpu::BufferUsage::STORAGE,
            label: Some("bounding sphere transform buffer"),
        });
        // todo should I drop existing?
        self.uniform_bind_group_registry.insert(model.id, self.create_model_uniform_bind_group(&instance_buffer, device, uniform_buffer));
    }

    fn create_instance_buffer(objects: &Vec<&Object>, device: &wgpu::Device, ) -> wgpu::Buffer {
        let instance_data = objects
            .iter()
            .map(|object| object.get_raw_transform())
            .collect::<Vec<_>>();
        let t = bytemuck::cast_slice(&instance_data);
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: t,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            label: Some("instance buffer"),
        });
        instance_buffer
    }

    pub fn update_object(&mut self, object: &Object, queue: &wgpu::Queue) {
        let transform = vec![object.get_raw_transform()];
        let bytes: &[u8] = bytemuck::cast_slice(&transform);
        let offset = (object.instance_id * bytes.len()) as u64;
        queue.write_buffer(
            self.instance_buffer_registry.get(&object.model_id).unwrap(),
            offset,
            bytes,
        );
    }

    fn create_mesh_index_buffer(mesh: &model::Mesh, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsage::INDEX,
            label: Some("index buffer"),
        })
    }

    fn create_vertex_buffer(mesh: &model::Mesh, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsage::VERTEX,
            label: Some("vertex buffer"),
        })
    }

    fn create_model_uniform_bind_group(
        &self,
        instance_buffer: &wgpu::Buffer,
        device: &wgpu::Device,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: uniform_buffer,
                        offset: 0,
                        size: None,
                    },
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: instance_buffer,
                        offset: 0,
                        size: None,
                    },
                },
            ],
            label: Some("uniform_bind_group"),
        })
    }

    fn draw_model_instanced<'a: 'b, 'b>(
        &'a self,
        render_pass: &'b mut RenderPass<'a>,
        internal_model: &InternalModel,
    ) {
        for internal_mesh in internal_model.internal_meshes.iter() {
            self.draw_mesh_instanced(
                render_pass,
                internal_mesh,
                self.uniform_bind_group_registry.get(&internal_model.id).unwrap(),
                0..internal_model.num_of_instances as u32,
            );
        }
    }

    fn draw_mesh_instanced<'a: 'b, 'b>(
        &'a self,
        render_pass: &'b mut RenderPass<'a>,
        internal_mesh: &InternalMesh,
        uniform_bind_group: &'a wgpu::BindGroup,
        instances: Range<u32>,
    ) {
        let vertex_buffer = self.vertex_buffer_registry.get(&internal_mesh.id).unwrap();
        let index_buffer = self.index_buffer_registry.get(&internal_mesh.id).unwrap();
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, uniform_bind_group, &[]);
        render_pass.draw_indexed(0..internal_mesh.count as u32, 0, instances);
    }
}

impl render::Drawer for BoundingSpheresDrawer2 {
    fn draw<'a: 'b, 'b>(&'a self, render_pass: &'b mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        self.draw_model_instanced(render_pass, &self.internal_model);
    }
}

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
