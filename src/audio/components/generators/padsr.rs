use crate::{
    audio::{
        components::component::{Component, ComponentType, StreamInfo},
        shared_audio_buffer::SharedAudioBuffer,
    },
    runtime::{instrument::VariableType, value::Value},
};

use super::generator::Generator;

#[derive(Clone)]
pub struct Padsr {
    sample_clock: f32,
}

impl Padsr {
    pub fn new() -> Self {
        Padsr { sample_clock: 0.0 }
    }
}

impl Component for Padsr {
    fn arg_count(&self) -> usize {
        Self::INPUT_TYPES.len()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Generator
    }

    fn process(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> Value {
        let mut buffer = SharedAudioBuffer::new(1, stream_info.buffer_size);

        let attack = args[0].get_float() * stream_info.sample_rate as f32;
        let decay = args[1].get_float() * stream_info.sample_rate as f32;
        let sustain_level = args[2].get_float();
        let release = args[3].get_float() * stream_info.sample_rate as f32;
        let total = args[4].get_float() * stream_info.sample_rate as f32;

        for sample in 0..stream_info.buffer_size {
            if self.sample_clock < attack {
                // attack phase
                buffer.set_sample(0, sample, self.sample_clock / attack);
            } else if (self.sample_clock - attack) < decay {
                // decay phase
                let base = self.sample_clock - attack;
                let level = 1.0 - (base / decay);
                buffer.set_sample(
                    0,
                    sample,
                    sustain_level + ((1.0 - sustain_level) * level),
                );
            } else if (self.sample_clock >= attack + decay) && (self.sample_clock < total - release)
            {
                // sustain phase
                buffer.set_sample(0, sample, sustain_level);
            } else if (self.sample_clock >= total - release)
                && (self.sample_clock - (total - release) < release)
            {
                // release phase
                let base = self.sample_clock - (total - release);
                let level = 1.0 - (base / release);
                buffer.set_sample(0, sample, sustain_level * level);
            } else {
                // after release
                buffer.set_sample(0, sample, 0.0);
            }
            self.sample_clock += 1.0;
        }

        Value::audio(buffer)
    }
}

impl Generator<5> for Padsr {
    const INPUT_TYPES: [VariableType; 5] = [
        VariableType::Float,
        VariableType::Float,
        VariableType::Float,
        VariableType::Float,
        VariableType::Float,
    ];
    const OUTPUT_TYPE: VariableType = VariableType::Audio;
}
