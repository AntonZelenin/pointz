#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Light {
    position: cgmath::Vector3<f32>,
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding: u32,
    color: cgmath::Vector3<f32>,
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding2: u32,
}

unsafe impl bytemuck::Zeroable for Light {}
unsafe impl bytemuck::Pod for Light {}

impl Light {
    pub fn new(position: cgmath::Vector3<f32>, color: cgmath::Vector3<f32>) -> Self {
        Light {
            position,
            _padding: 0,
            color,
            _padding2: 0,
        }
    }
}

// pub trait DrawLight<'a, 'b>
// where
//     'b: 'a,
// {
//     fn draw_light_mesh(
//         &mut self,
//         mesh: &'b Mesh,
//         uniforms: &'b wgpu::BindGroup,
//         light: &'b wgpu::BindGroup,
//     );
//     fn draw_light_mesh_instanced(
//         &mut self,
//         mesh: &'b Mesh,
//         instances: Range<u32>,
//         uniforms: &'b wgpu::BindGroup,
//         light: &'b wgpu::BindGroup,
//     ) where
//         'b: 'a;
//
//     fn draw_light_model(
//         &mut self,
//         model: &'b Model,
//         uniforms: &'b wgpu::BindGroup,
//         light: &'b wgpu::BindGroup,
//     );
//     fn draw_light_model_instanced(
//         &mut self,
//         model: &'b Model,
//         instances: Range<u32>,
//         uniforms: &'b wgpu::BindGroup,
//         light: &'b wgpu::BindGroup,
//     );
// }
//
// impl<'a, 'b> DrawLight<'a, 'b> for wgpu::RenderPass<'a>
// where
//     'b: 'a,
// {
//     fn draw_light_mesh(
//         &mut self,
//         mesh: &'b Mesh,
//         uniforms: &'b wgpu::BindGroup,
//         light: &'b wgpu::BindGroup,
//     ) {
//         self.draw_light_mesh_instanced(mesh, 0..1, uniforms, light);
//     }
//
//     fn draw_light_mesh_instanced(
//         &mut self,
//         mesh: &'b Mesh,
//         instances: Range<u32>,
//         uniforms: &'b wgpu::BindGroup,
//         light: &'b wgpu::BindGroup,
//     ) {
//         self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
//         self.set_index_buffer(mesh.index_buffer.slice(..));
//         self.set_bind_group(0, uniforms, &[]);
//         self.set_bind_group(1, light, &[]);
//         self.draw_indexed(0..mesh.num_elements, 0, instances);
//     }
//
//     fn draw_light_model(
//         &mut self,
//         model: &'b Model,
//         uniforms: &'b wgpu::BindGroup,
//         light: &'b wgpu::BindGroup,
//     ) {
//         self.draw_light_model_instanced(model, 0..1, uniforms, light);
//     }
//     fn draw_light_model_instanced(
//         &mut self,
//         model: &'b Model,
//         instances: Range<u32>,
//         uniforms: &'b wgpu::BindGroup,
//         light: &'b wgpu::BindGroup,
//     ) {
//         for mesh in &model.meshes {
//             self.draw_light_mesh_instanced(mesh, instances.clone(), uniforms, light);
//         }
//     }
// }
