use crate::renderer::render::{ResourceRegistry, ModelHandle, NewObjectHandle};
use crate::model::Model;
use crate::app::IndexDriver;
use glam::Vec3A;
use legion::Entity;
use std::collections::HashMap;
use cgmath::{Matrix4, Vector3, Quaternion};

pub struct NewObject {
    model_handle: ModelHandle,
    pub(crate) instance_id: usize,
    pub transform: Transform,
    entities: Vec<Entity>,
    // todo this is temporary
    pub(crate) handle: NewObjectHandle,
}

impl NewObject {
    // todo improve
    pub fn to_raw_instance(&self) -> TransformRaw {
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

impl Transform {
    pub fn to_raw(&self) -> TransformRaw {
        TransformRaw {
            model: Matrix4::from_translation(self.position) * Matrix4::from(self.rotation),
        }
    }
}

unsafe impl bytemuck::Pod for TransformRaw {}
unsafe impl bytemuck::Zeroable for TransformRaw {}

pub struct Manager {
    index_driver: IndexDriver,
    model_registry: ResourceRegistry<Model>,
    object_registry: HashMap<usize, NewObject>,
    model_instances: HashMap<usize, Vec<usize>>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            // todo mode it inside ResourceRegistry
            index_driver: IndexDriver::new(),
            model_registry: ResourceRegistry::new(),
            object_registry: HashMap::new(),
            model_instances: HashMap::new(),
        }
    }

    pub fn add_model(&mut self, model: Model) -> ModelHandle {
        let handle = ModelHandle(self.index_driver.next_id());
        self.model_registry.insert(handle.0, model);
        self.model_instances.insert(handle.0, vec![]);
        handle
    }

    pub fn get_model(&self, model_handle: &ModelHandle) -> &Model {
        self.model_registry.get(model_handle.0)
    }

    pub fn create_object(&mut self, model_handle: ModelHandle, transform: Transform) -> NewObjectHandle {
        let handle = NewObjectHandle(self.index_driver.next_id());
        let instance_id = self.object_registry.len();
        let object = NewObject {
            model_handle,
            instance_id,
            transform,
            entities: vec![],
            handle,
        };
        self.object_registry.insert(handle.0, object);
        self.model_instances.get_mut(&model_handle.0).unwrap().push(handle.0);
        handle
    }

    pub fn get_model_instances(&self, model_handle: &ModelHandle) -> Vec<&NewObject> {
        let obj_ids = self.model_instances.get(&model_handle.0).unwrap();
        obj_ids.iter().map(|id| self.object_registry.get(id).unwrap()).collect()
    }

    pub fn get_objects(&mut self) -> &mut HashMap<usize, NewObject> {
        &mut self.object_registry
    }
}
