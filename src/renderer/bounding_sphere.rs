use crate::renderer::render::Drawer;
use crate::model::{SimpleVertex, Vertex};
use crate::renderer::render;
use crate::model;
use crate::model::primitives::bounding_sphere;
use crate::scene::manager::Object;
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::util::DeviceExt;
use iced_wgpu::wgpu::RenderPass;
use cgmath::num_traits::Pow;
use ordered_float::OrderedFloat;
use std::collections::HashMap;

pub struct BoundingSpheresDrawer {
    model_ids: Vec<usize>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_bind_group_registry: HashMap<usize, wgpu::BindGroup>,
    num_mesh_indices: HashMap<usize, usize>,
    num_instances: HashMap<usize, usize>,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    instance_buffer_registry: HashMap<usize, wgpu::Buffer>,
}

impl BoundingSpheresDrawer {
    pub fn new(device: &wgpu::Device) -> BoundingSpheresDrawer {
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
        let mesh = bounding_sphere::get_mesh();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsage::INDEX,
            label: Some("index buffer"),
        });

        BoundingSpheresDrawer {
            model_ids: vec![],
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer_registry: HashMap::new(),
            uniform_bind_group_registry: HashMap::new(),
            num_mesh_indices: HashMap::new(),
            num_instances: HashMap::new(),
            uniform_bind_group_layout,
        }
    }

    // todo try to store a link to a device? renderer will always live longer than  specific renderers
    pub fn add(&mut self, model: &model::Model, instances: &Vec<&Object>, device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) {
        self.model_ids.push(model.id);
        let mut transforms: Vec<Transform> = vec![];
        for instance in instances.iter() {
            transforms.push(Transform {
                center: [instance.transform.position.x, instance.transform.position.y, instance.transform.position.z],
                radius: calc_bounding_sphere_radius(&model),
            });
        }
        self.num_instances.insert(model.id, instances.len());
        // todo should I drop existing buffer?
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&transforms),
            usage: wgpu::BufferUsage::STORAGE,
            label: Some("bounding sphere transform buffer"),
        });
        self.uniform_bind_group_registry.insert(model.id, device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                        buffer: &instance_buffer,
                        offset: 0,
                        size: None,
                    },
                }
            ],
            label: Some("bounding spheres uniform bind group"),
        }));
    }
}

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

impl Drawer for BoundingSpheresDrawer {
    fn draw<'a: 'b, 'b>(&'a self, render_pass: &'b mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        for model_id in self.model_ids.iter() {
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, self.uniform_bind_group_registry.get(&model_id).unwrap(), &[]);
            render_pass.draw(0..*self.num_instances.get(&model_id).unwrap() as u32, 0..1);
        }
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
