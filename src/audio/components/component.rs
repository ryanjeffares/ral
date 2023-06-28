use dyn_clone::{clone_trait_object, DynClone};

use crate::runtime::value::Value;

pub struct StreamInfo {
    pub sample_rate: u32,
    pub buffer_size: usize,
    pub channels: usize,
}

pub enum ComponentType {
    Generator,
}

pub trait Component: DynClone {
    fn arg_count(&self) -> usize;
    fn component_type(&self) -> ComponentType;
    fn process(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> Value;
}

clone_trait_object!(Component);
