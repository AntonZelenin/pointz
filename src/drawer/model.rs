use crate::drawer::render;
use crate::drawer::render::{MaterialHandle, MeshHandle, ObjectHandle, ResourceRegistry};
use crate::lighting::Light;
use crate::texture::TextureType;
use crate::{instance, model, texture};
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::util::DeviceExt;
use iced_wgpu::wgpu::RenderPass;
use std::ops::Range;

// todo use traits to remove deps?

pub struct Object {
    pub handle: ObjectHandle,
    pub instances: Vec<instance::Instance>,
}

struct InternalMesh {
    handle: MeshHandle,
    count: usize,
    material_handle: MaterialHandle,
}

struct InternalObject {
    handle: ObjectHandle,
    instances: Vec<instance::Instance>,
    internal_meshes: Vec<InternalMesh>,
}

struct IndexDriver {
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

pub struct ModelDrawer {
    index_driver: IndexDriver,
    pub render_pipeline: wgpu::RenderPipeline,
    pub light_bind_group: wgpu::BindGroup,
    objects: Vec<InternalObject>,
    material_bind_group_registry: ResourceRegistry<wgpu::BindGroup>,
    uniform_bind_group_registry: ResourceRegistry<wgpu::BindGroup>,
    vertex_buffer_registry: ResourceRegistry<wgpu::Buffer>,
    index_buffer_registry: ResourceRegistry<wgpu::Buffer>,
    instance_buffer_registry: ResourceRegistry<wgpu::Buffer>,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl ModelDrawer {
    pub fn new(device: &wgpu::Device) -> ModelDrawer {
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
                device.create_shader_module(wgpu::include_spirv!("../shader/spv/shader.vert.spv"));
            let fs_module =
                device.create_shader_module(wgpu::include_spirv!("../shader/spv/shader.frag.spv"));
            render::build_render_pipeline(&device, &render_pipeline_layout, vs_module, fs_module)
        };
        // todo light seems like it needs to be moved to a different drawer (¬_¬)
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
                resource: wgpu::BindingResource::Buffer(light_buffer.slice(..)),
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
            objects: vec![],
            material_bind_group_registry: ResourceRegistry::new(),
            uniform_bind_group_registry: ResourceRegistry::new(),
            vertex_buffer_registry: ResourceRegistry::new(),
            index_buffer_registry: ResourceRegistry::new(),
            instance_buffer_registry: ResourceRegistry::new(),
            uniform_bind_group_layout,
            texture_bind_group_layout,
        }
    }

    pub fn add_model(
        &mut self,
        model: model::Model,
        instances: Vec<instance::Instance>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniform_buffer: &wgpu::Buffer,
    ) -> Object {
        let object_handle = ObjectHandle(self.index_driver.next_id());
        let mut internal_meshes: Vec<InternalMesh> = vec![];
        let material_ids = self.create_material_bind_groups(&model, &device, &queue);
        for mesh in model.meshes.iter() {
            let mesh_handle = MeshHandle(self.index_driver.next_id());
            self.index_buffer_registry
                .insert(mesh_handle.0, self.create_mesh_index_buffer(&mesh, device));
            self.vertex_buffer_registry
                .insert(mesh_handle.0, self.create_mesh_vertex_buffer(mesh, device));
            let material_handle = MaterialHandle(material_ids[mesh.material_id]);
            internal_meshes.push(InternalMesh {
                count: mesh.indices.len(),
                handle: mesh_handle,
                material_handle,
            });
        }
        let instance_buffer = self.create_instance_buffer(&instances, device);
        self.uniform_bind_group_registry.insert(
            object_handle.0,
            self.create_model_uniform_bind_group(&instance_buffer, device, uniform_buffer),
        );
        self.instance_buffer_registry.insert(object_handle.0, instance_buffer);
        self.objects.push(InternalObject {
            handle: object_handle.clone(),
            instances: instances.clone(),
            internal_meshes,
        });
        Object {
            handle: object_handle,
            instances,
        }
    }

    fn create_instance_buffer(&mut self, instances: &Vec<instance::Instance>, device: &wgpu::Device) -> wgpu::Buffer {
        let instance_data = instances
            .iter()
            .map(instance::Instance::to_raw)
            .collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            label: Some("instance buffer"),
        });
        instance_buffer
    }

    pub fn update_instance(&mut self, handle: ObjectHandle, instance_idx: usize, instance: &instance::Instance, queue: &wgpu::Queue) {
        let raw = vec![instance.to_raw()];
        let bytes: &[u8] = bytemuck::cast_slice(&raw);
        let offset = (instance_idx * bytes.len()) as u64;
        queue.write_buffer(
            self.instance_buffer_registry.get(handle.0),
            offset,
            bytes,
        );
    }

    pub fn update(&mut self, object: Object, queue: &wgpu::Queue) {
        // todo duplicate
        let instance_data = object.instances
            .iter()
            .map(instance::Instance::to_raw)
            .collect::<Vec<_>>();
        queue.write_buffer(
            self.instance_buffer_registry.get(object.handle.0),
            0,
            bytemuck::cast_slice(&instance_data),
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

    fn create_mesh_vertex_buffer(&self, mesh: &model::Mesh, device: &wgpu::Device) -> wgpu::Buffer {
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
        instance_buffer: &wgpu::Buffer,
        device: &wgpu::Device,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.uniform_bind_group_layout,
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
        })
    }

    fn create_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
        })
    }

    fn create_texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
        })
    }

    fn create_light_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
        })
    }

    fn draw_model_instanced<'a: 'b, 'b>(
        &'a self,
        render_pass: &'b mut RenderPass<'a>,
        object: &InternalObject,
        instances: &Range<u32>,
    ) {
        for internal_mesh in object.internal_meshes.iter() {
            self.draw_mesh_instanced(
                render_pass,
                internal_mesh,
                self.uniform_bind_group_registry.get(object.handle.0),
                instances.clone(),
            );
        }
    }

    fn draw_mesh_instanced<'a: 'b, 'b>(
        &'a self,
        render_pass: &'b mut RenderPass<'a>,
        mesh: &InternalMesh,
        uniform_bind_group: &'a wgpu::BindGroup,
        instances: Range<u32>,
    ) {
        let vertex_buffer = self.vertex_buffer_registry.get(mesh.handle.0);
        let index_buffer = self.index_buffer_registry.get(mesh.handle.0);
        let material_bind_group = self
            .material_bind_group_registry
            .get(mesh.material_handle.0);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..));
        render_pass.set_bind_group(0, uniform_bind_group, &[]);
        render_pass.set_bind_group(1, material_bind_group, &[]);
        render_pass.set_bind_group(2, &self.light_bind_group, &[]);
        render_pass.draw_indexed(0..mesh.count as u32, 0, instances);
    }
}

impl render::Drawer for ModelDrawer {
    fn draw<'a: 'b, 'b>(&'a self, render_pass: &'b mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        for object in self.objects.iter() {
            self.draw_model_instanced(render_pass, &object, &(0..object.instances.len() as u32));
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
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
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
