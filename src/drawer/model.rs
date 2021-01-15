use std::ops::Range;
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::RenderPass;
use crate::lighting::Light;
use crate::model::{Model};
use crate::drawer::render::{build_render_pipeline, Drawer};
use iced_wgpu::wgpu::util::DeviceExt;
use crate::model;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct ModelDrawer<'a> {
    pub render_pipeline: wgpu::RenderPipeline,
    pub model_data: &'a Vec<model::ModelData>,
    pub light_bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl<'a> ModelDrawer<'a> {
    pub fn build_model_drawer(
        device: &wgpu::Device,
        model_data: &'a Vec<ModelData>,
    ) -> ModelDrawer<'a> {
        let uniform_bind_group_layout = <ModelDrawer<'a>>::create_uniform_bind_group_layout(device);
        let texture_bind_group_layout = <ModelDrawer<'a>>::create_texture_bind_group_layout(device);
        let light_bind_group_layout = <ModelDrawer<'a>>::create_light_bind_group_layout(device);
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
            build_render_pipeline(&device, &render_pipeline_layout, vs_module, fs_module)
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
            render_pipeline,
            model_data,
            light_bind_group,
            texture_bind_group_layout,
            uniform_bind_group_layout,
        }
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

    fn draw_model_instanced<'b>(
        &'b self,
        render_pass: &'b mut RenderPass<'b>,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
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
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..));
        render_pass.set_bind_group(0, &uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &material.bind_group, &[]);
        render_pass.set_bind_group(2, &light, &[]);
        render_pass.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}

impl<'a> Drawer for ModelDrawer<'a> {
    fn draw<'b>(&'b self, render_pass: &'b mut RenderPass<'b>) {
        render_pass.set_pipeline(&self.render_pipeline);
        for model_data in self.model_data {
            self.draw_model_instanced(
                render_pass,
                &model_data.model,
                0..model_data.instances.len() as u32,
                &model_data.uniform_bind_group,
                &self.light_bind_group,
            );
        }
    }
}

pub struct ModelData {
    model_data: model::ModelData,
    uniform_bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    mesh: model::Mesh,
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

pub struct Material {
    material: model::Material,
    bind_group: wgpu::BindGroup,
}
