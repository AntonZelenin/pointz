use crate::renderer::render;
use crate::lighting::Light;
use crate::texture::TextureType;
use crate::{model, texture};
use crate::model::{ModelVertex, Vertex};
use crate::app::IndexDriver;
use crate::scene::manager::{Object, RawTransform};
use crate::renderer::render::{InternalModel, InternalMesh};
use wgpu;
use wgpu::util::DeviceExt;
use std::ops::Range;
use std::collections::HashMap;
use crate::renderer::buffer::DynamicBuffer;

pub struct ModelDrawer {
    index_driver: IndexDriver,
    render_pipeline: wgpu::RenderPipeline,
    light_bind_group: wgpu::BindGroup,
    models: HashMap<usize, InternalModel>,
    material_bind_group_registry: HashMap<usize, wgpu::BindGroup>,
    uniform_bind_group_registry: HashMap<usize, wgpu::BindGroup>,
    vertex_buffer_registry: HashMap<usize, wgpu::Buffer>,
    index_buffer_registry: HashMap<usize, wgpu::Buffer>,
    instance_buffer_registry: HashMap<usize, DynamicBuffer<RawTransform>>,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl ModelDrawer {
    pub fn new(device: &wgpu::Device, primitive_topology: wgpu::PrimitiveTopology) -> ModelDrawer {
        let uniform_bind_group_layout = <ModelDrawer>::create_uniform_bind_group_layout(device);
        let texture_bind_group_layout = <ModelDrawer>::create_texture_bind_group_layout(device);
        let light_bind_group_layout = <ModelDrawer>::create_light_bind_group_layout(device);
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
                device.create_shader_module(&wgpu::include_spirv!("../shader/spv/shader.vert.spv"));
            let fs_module =
                device.create_shader_module(&wgpu::include_spirv!("../shader/spv/shader.frag.spv"));
            render::build_render_pipeline(&device, &render_pipeline_layout, vs_module, fs_module, ModelVertex::desc(), primitive_topology)
        };
        let light = Light::new((2.0, 2.0, 2.0).into(), (1.0, 1.0, 1.0).into());
        // We'll want to update our lights position, so we use COPY_DST
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[light]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            label: Some("light buffer"),
        });
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &light_buffer,
                    offset: 0,
                    size: None,
                },
            }],
            label: None,
        });
        // // this pipeline just renders light model
        // let light_render_pipeline = {
        //     let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //         label: Some("light pipeline"),
        //         bind_group_layouts: &[&uniform_bind_group_layout, &light_bind_group_layout],
        //         push_constant_ranges: &[],
        //     });
        //     let vs_module =
        //         device.create_shader_module(wgpu::include_spirv!("../shader/spv/light.vert.spv"));
        //     let fs_module =
        //         device.create_shader_module(wgpu::include_spirv!("../shader/spv/light.frag.spv"));
        //     build_render_pipeline(&device, &layout, vs_module, fs_module)
        // };
        ModelDrawer {
            index_driver: IndexDriver::new(),
            render_pipeline,
            light_bind_group,
            models: HashMap::new(),
            material_bind_group_registry: HashMap::new(),
            uniform_bind_group_registry: HashMap::new(),
            vertex_buffer_registry: HashMap::new(),
            index_buffer_registry: HashMap::new(),
            instance_buffer_registry: HashMap::new(),
            uniform_bind_group_layout,
            texture_bind_group_layout,
        }
    }

    pub fn init_model(
        &mut self,
        model: &model::Model,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniform_buffer: &wgpu::Buffer,
    ) {
        let mut internal_meshes: Vec<InternalMesh> = vec![];
        let material_ids = self.create_material_bind_groups(model, &device, &queue);
        for mesh in model.meshes.iter() {
            let mesh_id = self.index_driver.next_id();
            self.index_buffer_registry
                .insert(mesh_id, self.create_mesh_index_buffer(&mesh, device));
            self.vertex_buffer_registry
                .insert(mesh_id, self.create_vertex_buffer(mesh, device));
            // todo do I need it? or I can return as it was
            let material_id = if material_ids.len() == 0 {
                None
            } else {
                Some(material_ids[mesh.material_id])
            };
            internal_meshes.push(InternalMesh {
                count: mesh.indices.len(),
                id: mesh_id,
                material_id,
            });
        }
        let instance_buffer = self.create_instance_buffer(&vec![], device, queue);
        self.instance_buffer_registry
            .insert(model.id, instance_buffer);
        self.uniform_bind_group_registry.insert(
            model.id,
            self.create_model_uniform_bind_group(model.id, device, uniform_buffer),
        );
        self.models.insert(model.id, InternalModel {
            id: model.id,
            num_of_instances: 0,
            internal_meshes,
        });
    }

    // todo maybe I should pass encoder here, so that if I add a lot of instances I will be able to call submit() once when all instances are added
    // todo potentially slow place
    pub fn add_instances(
        &mut self,
        model_id: usize,
        objects: &Vec<&Object>,
        device: &wgpu::Device,
        uniform_buffer: &wgpu::Buffer,
        queue: &wgpu::Queue
    ) {
        let instance_buffer = self.instance_buffer_registry.get_mut(&model_id).unwrap();
        let instance_data = objects
            .iter()
            .map(|object| object.get_raw_transform())
            .collect::<Vec<_>>();
        let encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        instance_buffer.append(device, encoder, queue, bytemuck::cast_slice(&instance_data));
        *self.uniform_bind_group_registry.get_mut(&model_id).unwrap() = self.create_model_uniform_bind_group(model_id, device, uniform_buffer);
        let model = self.models.get_mut(&model_id).unwrap();
        model.num_of_instances += objects.len();
    }

    fn create_instance_buffer(
        &mut self,
        objects: &Vec<&Object>,
        device: &wgpu::Device,
        queue: &wgpu::Queue
    ) -> DynamicBuffer<RawTransform> {
        let instance_data = objects
            .iter()
            .map(|object| object.get_raw_transform())
            .collect::<Vec<_>>();
        // todo 4, change or comment
        let mut instance_buffer: DynamicBuffer<RawTransform> = DynamicBuffer::with_capacity(device, 4 + instance_data.len() * 2, wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST| wgpu::BufferUsage::COPY_SRC);
        let encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        instance_buffer.append(
            device,
            encoder,
            queue,
            bytemuck::cast_slice(&instance_data)
        );
        instance_buffer
    }

    pub fn update_object(&mut self, object: &Object, queue: &wgpu::Queue) {
        let transform = vec![object.get_raw_transform()];
        let bytes: &[u8] = bytemuck::cast_slice(&transform);
        let offset = (object.instance_id * bytes.len()) as u64;
        queue.write_buffer(
            self.instance_buffer_registry.get(&object.model_id).unwrap().get_buffer(),
            offset,
            bytes,
        );
    }

    /// Returns ordered material ids, meshes will take actual id by index using it's mesh.material_id
    fn create_material_bind_groups(
        &mut self,
        model: &model::Model,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<usize> {
        let mut ids: Vec<usize> = vec![];
        for material in model.materials.iter() {
            let new_id = self.index_driver.next_id();
            let material_bind_group = self.create_material_bind_group(material, device, queue);
            self.material_bind_group_registry
                .insert(new_id, material_bind_group);
            ids.push(new_id);
        }
        ids
    }

    fn create_mesh_index_buffer(&self, mesh: &model::Mesh, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsage::INDEX,
            label: Some("index buffer"),
        })
    }

    fn create_vertex_buffer(&self, mesh: &model::Mesh, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsage::VERTEX,
            label: Some("vertex buffer"),
        })
    }

    fn create_material_bind_group(
        &mut self,
        material: &model::Material,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> wgpu::BindGroup {
        let layout = &self.texture_bind_group_layout;
        let diffuse_view = create_view(&material.diffuse_texture, device, queue);
        let normal_view = create_view(&material.normal_texture, device, queue);
        let diffuse_sampler = self.create_sampler(device);
        let normal_sampler = self.create_sampler(device);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_sampler),
                },
            ],
            label: Some(&material.name),
        })
    }

    fn create_sampler(&self, device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        })
    }

    // pub fn remove_model_instance(&mut self, model_batch: &model::ModelBatch) {
    //     let model_id = model_batch.model.get_id().to_string();
    //     self.bind_group_registry.remove(&model_id);
    // }

    fn create_model_uniform_bind_group(
        &self,
        model_id: usize,
        device: &wgpu::Device,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        let instance_buffer = self.instance_buffer_registry.get(&model_id).unwrap();
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
                        buffer: instance_buffer.get_buffer(),
                        offset: 0,
                        size: None,
                    },
                },
            ],
            label: Some("uniform_bind_group"),
        })
    }

    fn create_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
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
                },
            ],
            label: Some("uniform_bind_group_layout"),
        })
    }

    fn create_texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: false,
                    },
                    count: None,
                },
                // normal map
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: false,
                    },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        })
    }

    fn create_light_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: None,
        })
    }

    fn draw_model_instanced<'a: 'b, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
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
        render_pass: &'b mut wgpu::RenderPass<'a>,
        internal_mesh: &InternalMesh,
        uniform_bind_group: &'a wgpu::BindGroup,
        instances: Range<u32>,
    ) {
        let vertex_buffer = self.vertex_buffer_registry.get(&internal_mesh.id).unwrap();
        let index_buffer = self.index_buffer_registry.get(&internal_mesh.id).unwrap();
        if let Some(material_id) = internal_mesh.material_id {
            let material_bind_group = self
                .material_bind_group_registry
                .get(&material_id).unwrap();
            render_pass.set_bind_group(1, material_bind_group, &[]);
        }
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, uniform_bind_group, &[]);
        render_pass.set_bind_group(2, &self.light_bind_group, &[]);
        render_pass.draw_indexed(0..internal_mesh.count as u32, 0, instances);
    }
}

impl render::Drawer for ModelDrawer {
    fn draw<'a: 'b, 'b>(&'a self, render_pass: &'b mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        for (_, internal_model) in self.models.iter() {
            self.draw_model_instanced(render_pass, internal_model);
        }
    }
}

pub fn create_view(
    texture: &texture::Texture,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> wgpu::TextureView {
    let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&texture.label),
        size: wgpu::Extent3d {
            width: texture.dimensions.0,
            height: texture.dimensions.1,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: match texture.type_ {
            TextureType::Normal => wgpu::TextureFormat::Rgba8Unorm,
            TextureType::Diffuse => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureType::Depth => texture::DEPTH_FORMAT,
        },
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    });
    if let Some(rgba_image) = texture.rgba_image.as_ref() {
        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            rgba_image,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * texture.dimensions.0,
                rows_per_image: texture.dimensions.1,
            },
            wgpu::Extent3d {
                width: texture.dimensions.0,
                height: texture.dimensions.1,
                depth: 1,
            },
        );
    }
    wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default())
}

pub fn create_depth_view(
    texture: &texture::Texture,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> wgpu::TextureView {
    let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&texture.label),
        size: wgpu::Extent3d {
            width: texture.dimensions.0,
            height: texture.dimensions.1,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: texture::DEPTH_FORMAT,
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT
            | wgpu::TextureUsage::SAMPLED
            | wgpu::TextureUsage::COPY_SRC,
    });
    if let Some(rgba_image) = texture.rgba_image.as_ref() {
        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            rgba_image,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * texture.dimensions.0,
                rows_per_image: texture.dimensions.1,
            },
            wgpu::Extent3d {
                width: texture.dimensions.0,
                height: texture.dimensions.1,
                depth: 1,
            },
        );
    }
    wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default())
}
