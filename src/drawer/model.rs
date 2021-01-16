use crate::drawer::render;
use crate::lighting::Light;
use crate::{model, instance, texture};
use std::ops::Range;
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::RenderPass;
use iced_wgpu::wgpu::util::DeviceExt;
use std::collections::HashMap;
use crate::model::ID;

pub struct ModelDrawer<'a> {
    device: &'a wgpu::Device,
    pub render_pipeline: wgpu::RenderPipeline,
    pub light_bind_group: wgpu::BindGroup,
    model_data: HashMap<String, &'a model::ModelData>,
    bing_groups: HashMap<String, wgpu::BindGroup>,
    vertex_buffers: HashMap<String, wgpu::Buffer>,
    index_buffers: HashMap<String, wgpu::Buffer>,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: &'a wgpu::Buffer,
    texture_drawer: TextureDrawer,
}

pub struct TextureDrawer {
    bind_group_layout: wgpu::BindGroupLayout,
    views: HashMap<String, wgpu::TextureView>,
    samplers: HashMap<String, wgpu::Sampler>,
}

impl<'a> ModelDrawer<'a> {
    pub fn build_model_drawer(device: &'a wgpu::Device, uniform_buffer: &'a wgpu::Buffer) -> ModelDrawer<'a> {
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
        let texture_drawer = TextureDrawer {
            bind_group_layout: texture_bind_group_layout,
            views: HashMap::new(),
            samplers: HashMap::new(),
        };
        ModelDrawer {
            device,
            render_pipeline,
            light_bind_group,
            model_data: HashMap::new(),
            bing_groups: HashMap::new(),
            vertex_buffers: HashMap::new(),
            index_buffers: HashMap::new(),
            uniform_bind_group_layout,
            uniform_buffer,
            texture_drawer,
        }
    }

    pub fn add_models(&mut self, models_data: &'a Vec<model::ModelData>) {
        for model_data in models_data.iter() {
            let model = &model_data.model;
            self.model_data.insert(
                model.get_id().to_string(),
                model_data
            );
            self.bing_groups.insert(
                model.get_id().to_string(),
                self.create_model_uniform_bind_group(model_data)
            );
            for mesh in model.meshes.iter() {
                self.index_buffers.insert(
                    mesh.get_id().to_string(),
                    self.create_mesh_index_buffer(&mesh)
                );
                self.vertex_buffers.insert(
                    mesh.get_id().to_string(),
                    self.create_mesh_vertex_buffer(mesh)
                );
            }
            for material in model.materials.iter() {
                self.bing_groups.insert(
                    material.get_id().to_string(),
                    self.create_material_bind_group(material)
                );
            }
        }
    }

    fn create_mesh_index_buffer(&self, mesh: &model::Mesh) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsage::INDEX,
            label: Some("index buffer"),
        })
    }

    fn create_mesh_vertex_buffer(&self, mesh: &model::Mesh) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsage::VERTEX,
            label: Some("vertex buffer"),
        })
    }

    fn create_material_bind_group(&mut self, material: &model::Material) -> wgpu::BindGroup {
        let layout = &self.texture_drawer.bind_group_layout;
        self.texture_drawer.views.insert(
            material.diffuse_texture.get_id().to_string(),
            self.create_view(&material.diffuse_texture)
        );
        self.texture_drawer.views.insert(
            material.normal_texture.get_id().to_string(),
            self.create_view(&material.normal_texture)
        );
        self.texture_drawer.samplers.insert(
            material.diffuse_texture.get_id().to_string(),
            self.create_sampler(&material.diffuse_texture)
        );
        self.texture_drawer.samplers.insert(
            material.normal_texture.get_id().to_string(),
            self.create_sampler(&material.normal_texture)
        );
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        self.texture_drawer.views.get(material.diffuse_texture.get_id()).unwrap()
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(self.texture_drawer.samplers.get(material.diffuse_texture.get_id()).unwrap()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(self.texture_drawer.views.get(material.normal_texture.get_id()).unwrap()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(self.texture_drawer.samplers.get(material.normal_texture.get_id()).unwrap()),
                },
            ],
            label: Some(&material.name),
        })
    }

    fn create_view(&self, texture: &texture::Texture) -> wgpu::TextureView {
        let wgpu_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&texture.label),
            size: wgpu::Extent3d {
                width: texture.dimensions.0,
                height: texture.dimensions.1,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if texture.is_normal_map {
                wgpu::TextureFormat::Rgba8Unorm
            } else {
                wgpu::TextureFormat::Rgba8UnormSrgb
            },
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });
        wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_sampler(&self, texture: &texture::Texture) -> wgpu::Sampler {
        // todo use one for all?
        self.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        })
    }

    pub fn remove_models(&mut self, model_data: &model::ModelData) {
        let model_id = model_data.model.get_id().to_string();
        self.model_data.remove(&model_id);
        self.bing_groups.remove(&model_id);
    }

    fn create_model_uniform_bind_group(&self, model_data: &'a model::ModelData) -> wgpu::BindGroup {
        let instance_data = model_data.instances.iter().map(instance::Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            label: Some("instance buffer"),
        });
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(self.uniform_buffer.slice(..)),
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

    fn draw_model_instanced<'b>(
        &'b self,
        render_pass: &'b mut RenderPass<'b>,
        model: &'b model::Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        // todo queue.write_texture ???
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material_id];
            self.draw_mesh_instanced(
                render_pass,
                mesh,
                material,
                instances.clone(),
                uniforms,
                light,
            );
        }
    }

    fn draw_mesh_instanced<'b>(
        &'b self,
        render_pass: &'b mut RenderPass<'b>,
        mesh: &'b model::Mesh,
        material: &'b model::Material,
        instances: Range<u32>,
        uniform_bind_group: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        let vertex_buffer = self.vertex_buffers.get(mesh.get_id()).unwrap();
        let index_buffer = self.index_buffers.get(mesh.get_id()).unwrap();
        let bind_group = self.bing_groups.get(material.get_id()).unwrap();
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..));
        render_pass.set_bind_group(0, &uniform_bind_group, &[]);
        render_pass.set_bind_group(1, bind_group, &[]);
        render_pass.set_bind_group(2, &light, &[]);
        render_pass.draw_indexed(0..mesh.indices.len() as u32, 0, instances);
    }
}

impl render::Drawer for ModelDrawer<'_> {
    fn draw<'b>(&'b self, render_pass: &'b mut RenderPass<'b>) {
        render_pass.set_pipeline(&self.render_pipeline);
        for (_, model_data) in self.model_data.iter() {
            self.draw_model_instanced(
                render_pass,
                &model_data.model,
                0..model_data.instances.len() as u32,
                self.bing_groups.get(model_data.model.get_id()).unwrap(),
                &self.light_bind_group,
            );
        }
    }
}
