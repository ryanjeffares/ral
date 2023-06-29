use crate::audio::components::component::{Component, ComponentType, StreamInfo};
use crate::runtime::instrument::VariableType;
use crate::runtime::value::Value;

use super::generator::Generator;

#[derive(Clone)]
pub struct Mtof;

impl Component for Mtof {
    fn arg_count(&self) -> usize {
        Self::INPUT_TYPES.len()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Generator
    }

    fn process(&mut self, _: &StreamInfo, args: Vec<Value>) -> Value {
        let midi = args[0].get_int() as f32;
        Value::float(2.0f32.powf((midi - 69.0) / 12.0) * 440.0)
    }
}

impl Generator<1> for Mtof {
    const INPUT_TYPES: [VariableType; 1] = [VariableType::Int];
    const OUTPUT_TYPE: VariableType = VariableType::Float;
}
