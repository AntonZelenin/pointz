use cgmath::Vector3;

// todo ?

pub const NUM_INSTANCES_PER_ROW: u32 = 5;
pub const NUM_ROWS: u32 = 5;
pub const INSTANCE_DISPLACEMENT: Vector3<f32> = Vector3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.0,
    NUM_ROWS as f32 * 0.5,
);
