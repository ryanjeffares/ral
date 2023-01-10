use rand::{rngs::ThreadRng, Rng};

use super::generator::{Generator, StreamInfo};
use crate::audio::{audio_buffer::AudioBuffer, components::component::Component};
use crate::runtime::value::Value;

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
    const PERF_ARG_COUNT: usize = 1;
}

impl Generator for Noise {
    fn get_next_audio_block(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> AudioBuffer {
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
        buffer
    }
}
