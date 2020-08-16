use cgmath::prelude::*;
use cgmath::{Matrix4, Vector4};
use crate::camera::{Camera, Projection};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Uniforms {
    view_position: Vector4<f32>,
    view_proj: Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_position: Zero::zero(),
            view_proj: Matrix4::identity(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous();
        self.view_proj = projection.calc_matrix() * camera.calc_matrix()
    }
}
