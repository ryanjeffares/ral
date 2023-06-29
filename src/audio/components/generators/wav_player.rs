use std::{cell::OnceCell, collections::HashMap, sync::Mutex};

use hound::WavReader;

use crate::{
    audio::{
        components::component::{Component, ComponentType, StreamInfo},
        shared_audio_buffer::SharedAudioBuffer,
    },
    runtime::{instrument::VariableType, value::Value},
};

use super::generator::Generator;

static SAMPLE_LOOKUP: Mutex<OnceCell<HashMap<String, (usize, Vec<f32>)>>> =
    Mutex::new(OnceCell::new());

#[derive(Clone)]
pub struct WavPlayer {
    index: usize,
}

impl WavPlayer {
    pub fn new() -> Self {
        WavPlayer { index: 0 }
    }
}

impl Component for WavPlayer {
    fn arg_count(&self) -> usize {
        Self::INPUT_TYPES.len()
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Generator
    }

    fn process(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> Value {
        let sample_path = args[0].get_string();
        let mut sample_lookup = SAMPLE_LOOKUP.lock().unwrap();
        sample_lookup.get_or_init(|| HashMap::new());
        let sample_lookup = sample_lookup.get_mut().unwrap();

        if !sample_lookup.contains_key(sample_path) {
            let mut reader = WavReader::open(&sample_path).unwrap();
            let spec = reader.spec();
            let channels = spec.channels as usize;

            match spec.sample_format {
                hound::SampleFormat::Float => {
                    let reader_samples = reader.samples::<f32>();
                    let mut samples = Vec::<f32>::with_capacity(reader_samples.len());
                    for sample in reader_samples {
                        match sample {
                            Ok(sample) => samples.push(sample),
                            Err(_) => samples.push(0.0),
                        }
                    }

                    sample_lookup.insert(sample_path.clone(), (channels, samples));
                }
                hound::SampleFormat::Int => {
                    let reader_samples = reader.samples::<i32>();
                    let mut samples = Vec::<f32>::with_capacity(reader_samples.len());
                    for sample in reader_samples {
                        match sample {
                            Ok(sample) => samples.push(sample as f32 / i32::MAX as f32),
                            Err(_) => samples.push(0.0),
                        }
                    }

                    sample_lookup.insert(sample_path.clone(), (channels, samples));
                }
            }
        }

        let (channels, samples) = sample_lookup.get(sample_path).unwrap();
        let mut output = SharedAudioBuffer::new(1, stream_info.buffer_size);

        if *channels == 1 {
            for sample in 0..stream_info.buffer_size {
                output.set_sample(0, sample, samples[self.index]);
                self.index += 1;
            }
        } else {
            'outer: for sample in 0..stream_info.buffer_size {
                for _ in 0..*channels {
                    if self.index >= samples.len() {
                        break 'outer;
                    }
                    
                    output.add_sample(0, sample, samples[self.index]);
                    self.index += 1;
                }
            }
        }

        Value::audio(output)
    }
}

impl Generator<1> for WavPlayer {
    const INPUT_TYPES: [VariableType; 1] = [VariableType::String];
    const OUTPUT_TYPE: VariableType = VariableType::Audio;
}