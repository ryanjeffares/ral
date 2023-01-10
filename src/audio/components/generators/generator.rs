use crate::{
    audio::{audio_buffer::AudioBuffer, components::component::Component},
    runtime::value::Value,
};

pub struct StreamInfo {
    pub sample_rate: cpal::SampleRate,
    pub buffer_size: usize,
    pub channels: usize,
}

pub trait Generator: Component {
    fn get_next_audio_block(&mut self, stream_info: &StreamInfo, args: Vec<Value>) -> AudioBuffer;
}
