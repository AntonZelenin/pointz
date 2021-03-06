// pub struct Object {
//     transform: Transform,
//     model: Optional<Model>,
//     pub(crate) entities: Vec<Entity>,
// }
//
// pub struct Transform {
//     pub position: Vector3<f32>,
//     pub rotation: Quaternion<f32>,
// }
//
// impl Object {
//     pub fn new() -> Object {
//         let rendering_entity = RenderingEntity {
//             handle: 1,
//             instance_index: 10,
//         };
//         Object {
//             transform: Transform {
//                 position: Default::default(),
//                 rotation: Default::default(),
//             },
//             model: None,
//             entities: vec![
//                 world.push(rendering_entity),
//             ]
//         }
//     }
// }
//
// struct RenderingEntity {
//     pub handle: ObjectHandle,
//     pub instance_index: usize,
// }
//
// pub fn add_object() {
//
// }