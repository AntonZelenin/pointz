use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}
