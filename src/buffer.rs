use crate::camera::{Camera, Projection};
use cgmath::prelude::*;
use cgmath::Matrix4;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Uniforms {
    pub view_proj: Matrix4<f32>,
    view_position: cgmath::Vector4<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_proj: Matrix4::identity(),
            view_position: Zero::zero(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_proj = projection.calc_matrix() * camera.calc_view_matrix();
        // We don't specifically need homogeneous coordinates since we're just using
        // a vec3 in the shader. We're using Point3 for the camera.eye, and this is
        // the easiest way to convert to Vector4. We're using Vector4 because of
        // the uniforms 16 byte spacing requirement
        self.view_position = camera.position.to_homogeneous();
    }
}
