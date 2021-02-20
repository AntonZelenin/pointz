use cgmath::{Matrix4, Quaternion, Vector3};
use crate::drawer::render::ObjectHandle;
use legion::Entity;

pub const NUM_INSTANCES_PER_ROW: u32 = 5;
pub const NUM_ROWS: u32 = 5;
pub const INSTANCE_DISPLACEMENT: Vector3<f32> = Vector3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.0,
    NUM_ROWS as f32 * 0.5,
);

pub struct Object {
    pub handle: ObjectHandle,
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub(crate) components: Vec<Entity>,
}

#[derive(Clone)]
pub struct Instance {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: Matrix4::from_translation(self.position) * Matrix4::from(self.rotation),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceRaw {
    model: Matrix4<f32>,
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}
