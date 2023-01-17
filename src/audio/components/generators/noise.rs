use rand::{rngs::ThreadRng, Rng};

use super::generator::Generator;
use crate::audio::components::component::{ComponentType, StreamInfo};
use crate::audio::{audio_buffer::AudioBuffer, components::component::Component};
use crate::runtime::instrument::VariableType;
use crate::runtime::value::Value;

#[derive(Clone)]
pub struct Noise {
    rng: ThreadRng,
}

impl Noise {
    pub fn new() -> Self {
        Noise {
            rng: rand::thread_rng(),
        }
    }
}

impl Component for Noise {
    fn arg_count(&self) -> usize {
        Self::INPUT_TYPES.len()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Generator
    }

    fn process(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> Value {
        let mut buffer = AudioBuffer::new(stream_info.channels, stream_info.buffer_size);
        for sample in 0..stream_info.buffer_size {
            for channel in 0..stream_info.channels {
                buffer.set_sample(
                    channel,
                    sample,
                    self.rng.gen_range(-1.0..1.0) * args[0].get_float(),
                );
            }
        }

        Value::audio(buffer)
    }
}

impl Generator<1> for Noise {
    const INPUT_TYPES: [VariableType; 1] = [VariableType::Float];
    const OUTPUT_TYPE: VariableType = VariableType::Audio;
}
