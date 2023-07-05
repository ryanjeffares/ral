use crate::{
    audio::{
        components::component::{Component, ComponentType, StreamInfo},
        shared_audio_buffer::SharedAudioBuffer,
    },
    runtime::{instrument::VariableType, value::Value},
};
use sndfile::{self, OpenOptions, ReadOptions, SndFileIO};
use std::{cell::OnceCell, collections::HashMap, sync::Mutex};

use super::generator::Generator;

static SAMPLE_LOOKUP: Mutex<OnceCell<HashMap<String, (usize, Vec<f32>)>>> =
    Mutex::new(OnceCell::new());

#[derive(Clone)]
pub struct Sample {
    index: usize,
}

impl Sample {
    pub fn new() -> Self {
        let sl = SAMPLE_LOOKUP.lock().unwrap();
        sl.get_or_init(|| HashMap::new());

        Sample { index: 0 }
    }
}

impl Component for Sample {
    fn arg_count(&self) -> usize {
        Self::INPUT_TYPES.len()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Generator
    }

    fn process(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> Vec<Value> {
        let sample_path = args[0].get_string();

        let mut sample_lookup = SAMPLE_LOOKUP.lock().unwrap();
        let sample_lookup = sample_lookup.get_mut().unwrap();

        if !sample_lookup.contains_key(sample_path) {
            // new sample, load it in
            let mut snd = OpenOptions::ReadOnly(ReadOptions::Auto)
                .from_path(sample_path)
                .unwrap();
            let samples: Vec<f32> = match snd.read_all_to_vec() {
                Ok(samples) => samples,
                Err(err) => {
                    eprintln!("Failed to load {}: {:?}", sample_path, err);
                    vec![]
                }
            };

            println!("Opened file {sample_path}, read {} samples", samples.len());
            sample_lookup.insert(sample_path.clone(), (snd.get_channels(), samples));
        }

        let (channels, samples) = sample_lookup.get(sample_path).unwrap();
        let mut output = vec![Value::audio(SharedAudioBuffer::new(1, stream_info.buffer_size)); *channels];

        // this handles interleaved??
        'outer: for sample in 0..stream_info.buffer_size {
            for channel in 0..*channels {
                if self.index >= samples.len() {
                    break 'outer;
                }

                output[channel].get_audio_mut().add_sample(0, sample, samples[self.index]);
                self.index += 1;
            }
        }

        output
    }
}

impl Generator<1> for Sample {
    const INPUT_TYPES: [VariableType; 1] = [VariableType::String];
    const OUTPUT_TYPE: VariableType = VariableType::Audio;
}
