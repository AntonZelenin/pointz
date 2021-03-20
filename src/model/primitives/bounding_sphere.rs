use crate::model::{ModelVertex, Mesh};
use cgmath::Vector3;

pub fn get_mesh() -> Mesh {
    Mesh {
        name: "bounding sphere mesh".parse().unwrap(),
        vertices: vec![
            ModelVertex {
                position: Vector3::new(0.0, -1.0, 1.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(0.0, 1.0, 1.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(0.0, 1.0, -1.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(0.0, -1.0, -1.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(-1.0, -1.0, 0.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(-1.0, 1.0, 0.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(1.0, 1.0, 0.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(1.0, -1.0, 0.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(-1.0, 0.0, 1.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(1.0, 0.0, 1.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(1.0, 0.0, -1.0),
                ..Default::default()
            },
            ModelVertex {
                position: Vector3::new(-1.0, 0.0, -1.0),
                ..Default::default()
            },
        ],

        indices: vec![
            0, 1, 2,
            2, 3, 1,

            4, 5, 6,
            6, 7, 4,

            8, 9, 10,
            10, 11, 8,
        ],
        material_id: 0
    }
}
