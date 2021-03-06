use crate::renderer::render::Drawer;
use crate::model::{SimpleVertex, Vertex};
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::util::DeviceExt;
use iced_wgpu::wgpu::RenderPass;
use crate::renderer::render;
use crate::model;
use cgmath::num_traits::Pow;
use ordered_float::OrderedFloat;
use crate::object::Instance;

pub struct BoundingSpheresDrawer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buff: wgpu::Buffer,
    uniform_bind_group: Option<wgpu::BindGroup>,
    num_vertices: usize,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
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
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: Some("debug_uniform_bind_group_layout"),
            });
        // let empty_radius_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     contents: &[],
        //     usage: wgpu::BufferUsage::STORAGE,
        //     label: Some("bounding sphere radius buffer"),
        // });
        // let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &uniform_bind_group_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::Buffer {
        //                 buffer: &uniform_buffer,
        //                 offset: 0,
        //                 size: None,
        //             },
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Buffer {
        //                 buffer: &empty_radius_buffer,
        //                 offset: 0,
        //                 size: None,
        //             },
        //         }
        //     ],
        //     label: Some("debug_uniform_bind_group"),
        // });
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
            // todo point list cannot draw circles, it draws only points
            render::build_render_pipeline(device, &layout, vs_module, fs_module, SimpleVertex::desc(), wgpu::PrimitiveTopology::TriangleList)
        };
        let vertex_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: &[],
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        BoundingSpheresDrawer {
            render_pipeline,
            vertex_buff,
            uniform_bind_group: None,
            num_vertices: 0,
            uniform_bind_group_layout,
        }
    }

    // todo try to store a link to a device? renderer will always live longer than  specific renderers
    pub fn add(&mut self, model: &model::Model, instances: &Vec<Instance>, device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) {
        // todo it should write to the same buffer, now it creates a new one for each model
        let mut centers: Vec<SimpleVertex> = vec![];
        for instance in instances.iter() {
            centers.push(SimpleVertex {
                position: [instance.position.x, instance.position.y, instance.position.z],
            })
        }
        self.vertex_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&centers),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        let radius_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            // todo now it's the same for all instances, will be updated when models support scaling
            contents: bytemuck::cast_slice(&[calc_bounding_sphere_radius(&model)]),
            usage: wgpu::BufferUsage::STORAGE,
            label: Some("bounding sphere radius buffer"),
        });
        self.uniform_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                        buffer: &radius_buffer,
                        offset: 0,
                        size: None,
                    },
                }
            ],
            label: Some("debug_uniform_bind_group"),
        }));
    }
}

impl Drawer for BoundingSpheresDrawer {
    fn draw<'a: 'b, 'b>(&'a self, render_pass: &'b mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buff.slice(..));
        render_pass.set_bind_group(0, &self.uniform_bind_group.as_ref().unwrap(), &[]);
        // todo set vertices dynamically
        render_pass.draw(0..self.num_vertices as u32, 0..1);
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