use std::f32::consts::PI;

use super::generator::Generator;
use crate::audio::{
    audio_buffer::AudioBuffer,
    components::component::{Component, StreamInfo},
};
use crate::runtime::{instrument::VariableType, value::Value};

#[derive(Clone)]
pub struct Sine {
    sample_clock: f32,
}

impl Sine {
    pub fn new() -> Self {
        Sine { sample_clock: 0.0 }
    }
}

impl Component for Sine {
    fn arg_count(&self) -> usize {
        Self::INPUT_TYPES.len()
    }

    fn get_next_audio_block(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> AudioBuffer {
        let mut buffer = AudioBuffer::new(stream_info.channels, stream_info.buffer_size);
        let amps = args[0].get_float();
        let freq = args[1].get_float();
        let sr = stream_info.sample_rate.0 as f32;
        for sample in 0..stream_info.buffer_size {
            self.sample_clock = (self.sample_clock + 1.0) % sr;
            for channel in 0..stream_info.channels {
                buffer.set_sample(channel, sample, (self.sample_clock * freq * 2.0 * PI / sr).sin() * amps);
            }
        }
        buffer
    }
}

impl Generator<2> for Sine {
    const INPUT_TYPES: [VariableType; 2] = [VariableType::Float, VariableType::Float];
    const OUTPUT_TYPE: VariableType = VariableType::Audio;
}
