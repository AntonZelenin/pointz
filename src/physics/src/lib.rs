use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
struct BoundingSphere {
    center: Vec3,
    radius: f32,
}
