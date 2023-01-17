use std::f32::consts::PI;

use super::generator::Generator;
use crate::audio::{
    audio_buffer::AudioBuffer,
    components::component::{Component, ComponentType, StreamInfo},
};
use crate::runtime::{instrument::VariableType, value::Value};

pub enum Shape {
    Sine = 0,
    Saw = 1,
    Square = 2,
    Tri = 3,
}

#[derive(Clone)]
pub struct Oscil {
    phase: f32,
}

impl Oscil {
    pub fn new() -> Self {
        Oscil { phase: 0.0 }
    }
}

impl Component for Oscil {
    fn arg_count(&self) -> usize {
        Self::INPUT_TYPES.len()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Generator
    }

    fn process(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> Value {
        let mut buffer = AudioBuffer::new(stream_info.channels, stream_info.buffer_size);

        let amps = args[0].get_float();
        let freq = args[1].get_float();
        let shape: Shape = args[2].get_int().into();
        let sr = stream_info.sample_rate.0 as f32;

        for sample in 0..stream_info.buffer_size {
            let value = match shape {
                Shape::Sine => {
                    if self.phase >= 1.0 {
                        self.phase = 0.0;
                    }
                    let output = (self.phase * PI * 2.0).sin();
                    self.phase += 1.0 / (sr / freq);
                    output
                }
                Shape::Saw => {
                    if self.phase >= 1.0 {
                        self.phase = -1.0;
                    }
                    let output = self.phase;
                    self.phase += 1.0 / (sr / freq) * 2.0;
                    output
                }
                Shape::Square => {
                    if self.phase >= 1.0 {
                        self.phase = 0.0
                    }
                    self.phase += 1.0 / (sr / freq);
                    if self.phase < 0.5 {
                        -1.0
                    } else {
                        1.0
                    }
                }
                Shape::Tri => {
                    if self.phase >= 1.0 {
                        self.phase = 0.0;
                    }
                    self.phase += 1.0 / (sr / freq);
                    if self.phase < 0.5 {
                        (self.phase - 0.25) * 4.0
                    } else {
                        ((1.0 - self.phase) - 0.25) * 4.0
                    }
                }
            };
            for channel in 0..stream_info.channels {
                buffer.set_sample(channel, sample, value * amps);
            }
        }

        Value::audio(buffer)
    }
}

impl Generator<3> for Oscil {
    const INPUT_TYPES: [VariableType; 3] =
        [VariableType::Float, VariableType::Float, VariableType::Int];
    const OUTPUT_TYPE: VariableType = VariableType::Audio;
}

impl From<i64> for Shape {
    fn from(value: i64) -> Self {
        match value {
            0 => Shape::Sine,
            1 => Shape::Saw,
            2 => Shape::Square,
            3 => Shape::Tri,
            _ => panic!("No oscil shape for integer value {value}"),
        }
    }
}
