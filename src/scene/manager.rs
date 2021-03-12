use crate::model::Model;
use crate::app::MultipleIndexDriver;
use glam::Vec3A;
use legion::Entity;
use std::collections::HashMap;
use cgmath::{Matrix4, Vector3, Quaternion};

pub struct NewObject {
    // todo probably this is temporary
    pub(crate) id: usize,
    pub model_id: usize,
    pub(crate) instance_id: usize,
    pub transform: Transform,
    entities: Vec<Entity>,
}

impl NewObject {
    pub fn get_raw_transform(&self) -> TransformRaw {
        TransformRaw {
            model: Matrix4::from_translation(self.transform.position) * Matrix4::from(self.transform.rotation),
        }
    }
}

#[derive(Clone)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vec3A,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TransformRaw {
    model: Matrix4<f32>,
}

unsafe impl bytemuck::Pod for TransformRaw {}
unsafe impl bytemuck::Zeroable for TransformRaw {}

pub struct Manager {
    index_driver: MultipleIndexDriver,
    model_registry: HashMap<usize, Model>,
    object_registry: HashMap<usize, NewObject>,
    model_instances: HashMap<usize, Vec<usize>>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            index_driver: MultipleIndexDriver::new(),
            model_registry: HashMap::new(),
            object_registry: HashMap::new(),
            model_instances: HashMap::new(),
        }
    }

    pub fn add_model(&mut self, model: Model) {
        let model_id = model.id;
        self.model_registry.insert(model_id, model);
        self.model_instances.insert(model_id, vec![]);
    }

    pub fn get_model(&self, model_id: usize) -> &Model {
        self.model_registry.get(&model_id).unwrap()
    }

    pub fn get_model_ids(&self) -> Vec<usize> {
        self.model_registry.iter().map(|(_, m)| m.id).collect()
    }

    pub fn create_object(&mut self, model_id: usize, transform: Transform) -> usize {
        let id = self.index_driver.next_id(&model_id);
        let instance_id = match self.model_instances.get(&model_id) {
            Some(instances) => instances.len(),
            None => 0,
        };
        let object = NewObject {
            id,
            model_id,
            instance_id,
            transform,
            entities: vec![],
        };
        self.object_registry.insert(id, object);
        // todo why is it working? don't I need to init the vector?
        self.model_instances.get_mut(&model_id).unwrap().push(id);
        id
    }

    pub fn get_model_instances(&self, model_id: usize) -> Vec<&NewObject> {
        let obj_ids = self.model_instances.get(&model_id).unwrap();
        obj_ids.iter().map(|id| self.object_registry.get(id).unwrap()).collect()
    }

    pub fn get_objects(&mut self) -> &mut HashMap<usize, NewObject> {
        &mut self.object_registry
    }
}
