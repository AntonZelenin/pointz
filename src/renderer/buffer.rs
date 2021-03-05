use crate::camera::{Camera, Projection};
use cgmath::prelude::*;
use cgmath::Matrix4;
use iced_wgpu::wgpu;

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

pub struct DynamicBuffer<T: bytemuck::Pod + Copy> {
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsage,
    capacity: usize,
    len: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<T: bytemuck::Pod + Copy + 'static> DynamicBuffer<T> {
    pub fn with_capacity(
        device: &wgpu::Device,
        initial_capacity: usize,
        mut usage: wgpu::BufferUsage,
    ) -> Self {
        usage |= wgpu::BufferUsage::COPY_DST;
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                mapped_at_creation: false,
                label: None,
                size: (initial_capacity * std::mem::size_of::<T>()) as u64,
                usage,
            }),
            usage,
            capacity: initial_capacity,
            len: 0,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn append(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        data: &[T],
    ) {
        if self.len + data.len() > self.capacity {
            // todo it might eat a lot of memory if a buffer will be large
            let new_capacity = self.len * data.len() * 2;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                mapped_at_creation: false,
                label: None,
                size: (new_capacity * std::mem::size_of::<T>()) as u64,
                usage: self.usage,
            });
            self.capacity = new_capacity;

            encoder.copy_buffer_to_buffer(
                &self.buffer,
                0,
                &buffer,
                0,
                (data.len() * std::mem::size_of::<T>()) as u64,
            );
            self.buffer = buffer;
        }
        queue.write_buffer(&self.buffer, self.len as u64, bytemuck::cast_slice(data));
        self.len = self.len + data.len();
    }

    pub fn get_buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn len(&self) -> usize {
        self.len
    }
}
