use std::mem;
use std::path::Path;
use anyhow::*;
use cgmath::{Vector2, Vector3, Zero};
use iced_wgpu::wgpu;
use tobj::LoadOptions;
use crate::app::IndexDriver;
use crate::texture;

// todo move to render?
pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

// todo move to render?
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SimpleVertex {
    pub position: [f32; 3],
}

unsafe impl bytemuck::Pod for SimpleVertex {}

unsafe impl bytemuck::Zeroable for SimpleVertex {}

impl Vertex for SimpleVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SimpleVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array!(
                0 => Float32x3,
            ),
        }
    }
}

pub struct Model {
    pub id: usize,
    pub label: String,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture,
}

impl Material {
    pub fn new(
        name: &str,
        diffuse_texture: texture::Texture,
        normal_texture: texture::Texture,
    ) -> Material {
        Material {
            name: String::from(name),
            diffuse_texture,
            normal_texture,
        }
    }
}

pub struct Mesh {
    pub name: String,
    pub vertices: Vec<ModelVertex>,
    pub indices: Vec<u32>,
    pub material_id: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ModelVertex {
    pub position: Vector3<f32>,
    pub tex_coords: Vector2<f32>,
    pub normal: Vector3<f32>,
    pub tangent: Vector3<f32>,
    pub bitangent: Vector3<f32>,
}

unsafe impl bytemuck::Pod for ModelVertex {}

unsafe impl bytemuck::Zeroable for ModelVertex {}

impl Default for ModelVertex {
    fn default() -> Self {
        Self {
            position: Vector3::zero(),
            tex_coords: Vector2::zero(),
            normal: Vector3::zero(),
            tangent: Vector3::zero(),
            bitangent: Vector3::zero(),
        }
    }
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Loader {
    index_driver: IndexDriver,
}

impl Loader {
    pub fn new() -> Self {
        Self {
            index_driver: IndexDriver::new(),
        }
    }

    pub fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<Model> {
        let (obj_models, obj_materials) = tobj::load_obj(
            path.as_ref(),
            &LoadOptions{
                single_index: false,
                triangulate: true,
                ignore_points: false,
                ignore_lines: false,
            },
        )?;

        // We're assuming that the texture files are stored with the obj file
        let containing_folder = path.as_ref().parent().unwrap();

        let mut materials = Vec::new();
        for mat in obj_materials.unwrap() {
            let diffuse_path = mat.diffuse_texture;
            let diffuse_texture =
                texture::Texture::load(containing_folder.join(diffuse_path), false)?;

            let normal_path = mat.normal_texture;
            let normal_texture = texture::Texture::load(containing_folder.join(normal_path), true)?;

            materials.push(Material::new(&mat.name, diffuse_texture, normal_texture));
        }

        let mut meshes = Vec::new();
        for m in obj_models {
            let mut vertices = Vec::new();
            for i in 0..m.mesh.positions.len() / 3 {
                let tex_coords: Vector2<f32> = if m.mesh.texcoords.len() == 0 {
                    Vector2::new(0.0, 0.0)
                } else {
                    Vector2::new(m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1])
                };
                let normal: Vector3<f32> = if m.mesh.normals.len() == 0 {
                    Vector3::new(1.0, 1.0, 1.0)
                } else {
                    Vector3::new(
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    )
                };
                vertices.push(ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ]
                        .into(),
                    // tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]].into(),
                    tex_coords,
                    // normal: [
                    //     m.mesh.normals[i * 3],
                    //     m.mesh.normals[i * 3 + 1],
                    //     m.mesh.normals[i * 3 + 2],
                    // ]
                    //     .into(),
                    normal,
                    tangent: [0.0; 3].into(),
                    bitangent: [0.0; 3].into(),
                });
            }

            let indices = &m.mesh.indices;

            // Calculate tangents and bitangets. We're going to
            // use the triangles, so we need to loop through the
            // indices in chunks of 3
            for c in indices.chunks(3) {
                let v0 = vertices[c[0] as usize];
                let v1 = vertices[c[1] as usize];
                let v2 = vertices[c[2] as usize];

                let pos0 = v0.position;
                let pos1 = v1.position;
                let pos2 = v2.position;

                let uv0 = v0.tex_coords;
                let uv1 = v1.tex_coords;
                let uv2 = v2.tex_coords;

                // Calculate the edges of the triangle
                let delta_pos1 = pos1 - pos0;
                let delta_pos2 = pos2 - pos0;

                // This will give us a direction to calculate the
                // tangent and bitangent
                let delta_uv1 = uv1 - uv0;
                let delta_uv2 = uv2 - uv0;

                // Solving the following system of equations will
                // give us the tangent and bitangent.
                //     delta_pos1 = delta_uv1.x * T + delta_u.y * B
                //     delta_pos2 = delta_uv2.x * T + delta_uv2.y * B
                // Luckily, the place I found this equation provided
                // the solution!
                let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
                let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;

                // We'll use the same tangent/bitangent for each vertex in the triangle
                vertices[c[0] as usize].tangent = tangent;
                vertices[c[1] as usize].tangent = tangent;
                vertices[c[2] as usize].tangent = tangent;

                vertices[c[0] as usize].bitangent = bitangent;
                vertices[c[1] as usize].bitangent = bitangent;
                vertices[c[2] as usize].bitangent = bitangent;
            }

            meshes.push(Mesh {
                name: m.name,
                vertices,
                indices: m.mesh.indices,
                material_id: m.mesh.material_id.unwrap_or(0),
            });
        }

        Ok(Model {
            id: self.index_driver.next_id(),
            label: String::from(path.as_ref().file_name().unwrap().to_str().unwrap()),
            meshes,
            materials,
        })
    }

    pub fn load_primitive<P: AsRef<Path>>(&mut self, path: P) -> Result<Model> {
        let (obj_models, obj_materials) = tobj::load_obj(
            path.as_ref(),
            &LoadOptions::default(),
        )?;

        // We're assuming that the texture files are stored with the obj file
        let containing_folder = path.as_ref().parent().unwrap();

        let mut materials = Vec::new();
        for mat in obj_materials.unwrap() {
            let diffuse_path = mat.diffuse_texture;
            let diffuse_texture =
                texture::Texture::load(containing_folder.join(diffuse_path), false)?;

            let normal_path = mat.normal_texture;
            let normal_texture = texture::Texture::load(containing_folder.join(normal_path), true)?;

            materials.push(Material::new(&mat.name, diffuse_texture, normal_texture));
        }

        let mut meshes = Vec::new();
        for m in obj_models {
            let mut vertices = Vec::new();
            for i in 0..m.mesh.positions.len() / 3 {
                let tex_coords: Vector2<f32> = Vector2::new(0.0, 0.0);
                let normal: Vector3<f32> = Vector3::new(1.0, 1.0, 1.0);
                vertices.push(ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ]
                        .into(),
                    tex_coords,
                    normal,
                    tangent: [0.0; 3].into(),
                    bitangent: [0.0; 3].into(),
                });
            }
            meshes.push(Mesh {
                name: m.name,
                vertices,
                indices: m.mesh.indices,
                material_id: m.mesh.material_id.unwrap_or(0),
            });
        }

        Ok(Model {
            id: self.index_driver.next_id(),
            label: String::from(path.as_ref().file_name().unwrap().to_str().unwrap()),
            meshes,
            materials,
        })
    }
}
