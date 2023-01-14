use dyn_clone::{DynClone, clone_trait_object};

use crate::{audio::audio_buffer::AudioBuffer, runtime::value::Value};

pub struct StreamInfo {
    pub sample_rate: cpal::SampleRate,
    pub buffer_size: usize,
    pub channels: usize,
}

pub trait Component: DynClone {
    fn arg_count(&self) -> usize;
    fn get_next_audio_block(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> AudioBuffer;
}

clone_trait_object!(Component);