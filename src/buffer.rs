use crate::camera::{Camera, Projection};
use cgmath::prelude::*;
use cgmath::Matrix4;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Uniforms {
    view_proj: Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_proj: Matrix4::identity(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_proj = projection.calc_matrix() * camera.calc_matrix()
    }
}
