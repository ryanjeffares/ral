use crate::{audio::components::component::{Component, ComponentType, StreamInfo}, runtime::{instrument::VariableType, value::Value}};

use super::generator::Generator;

#[derive(Clone)]
pub struct Adsr {
    sample_clock: f32,
}

impl Adsr {
    pub fn new() -> Self {
        Adsr { sample_clock: 0.0 }
    }
}

impl Component for Adsr {
    fn arg_count(&self) -> usize {
        Self::INPUT_TYPES.len()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Generator
    }

    fn process(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> Value {
        let output;

        let attack = args[0].get_float() * stream_info.sample_rate.0 as f32;
        let decay = args[1].get_float() * stream_info.sample_rate.0 as f32;
        let sustain_level = args[2].get_float();
        let release = args[3].get_float() * stream_info.sample_rate.0 as f32;
        let total = args[4].get_float() * stream_info.sample_rate.0 as f32;

        if self.sample_clock < attack {
            // attack phase
            output = self.sample_clock / attack;
        } else if (self.sample_clock - attack) < decay {
            // decay phase
            let base = self.sample_clock - attack;
            let level = 1.0 - (base / decay);
            output = sustain_level + ((1.0 - sustain_level) * level);            
        } else if (self.sample_clock >= attack + decay)
            && (self.sample_clock < total - release)
        {
            // sustain phase
            output = sustain_level;
        } else if (self.sample_clock >= total - release)
            && (self.sample_clock - (total - release) < release)
        {
            // release phase
            let base = self.sample_clock - (total - release);
            let level = 1.0 - (base / release);
            output =  sustain_level * level;
        } else {
            // after release
            output = 0.0;
        }

        self.sample_clock += stream_info.buffer_size as f32;
        Value::float(output)
    }
}

impl Generator<5> for Adsr {
    const INPUT_TYPES: [VariableType; 5] = [
        VariableType::Float,
        VariableType::Float,
        VariableType::Float,
        VariableType::Float,
        VariableType::Float,
    ];
    const OUTPUT_TYPE: VariableType = VariableType::Float;
}
