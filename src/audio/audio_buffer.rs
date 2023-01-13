use crate::utils::number_array::NumberArray;

#[derive(Clone)]
pub struct AudioBuffer {
    channels: usize,
    buffer_size: usize,
    data: Vec<NumberArray<f32>>,
}

impl AudioBuffer {
    pub fn new(channels: usize, buffer_size: usize) -> Self {
        AudioBuffer {
            channels,
            buffer_size,
            data: vec![NumberArray::<f32>::new(buffer_size); channels],
        }
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn clear(&mut self) {
        for channel in self.data.iter_mut() {
            channel.fill(0.0);
        }
    }

    pub fn get_sample(&self, channel: usize, sample: usize) -> f32 {
        self.data[channel][sample]
    }

    pub fn set_sample(&mut self, channel: usize, sample: usize, value: f32) {
        self.data[channel][sample] = value;
    }

    pub fn add_sample(&mut self, channel: usize, sample: usize, value: f32) {
        self.data[channel][sample] += value;
    }

    pub fn add_from(&mut self, source: &AudioBuffer) {
        assert!(self.channels == source.channels && self.buffer_size == source.buffer_size);
        for channel in 0..self.channels {
            for sample in 0..self.buffer_size {
                self.data[channel][sample] += source.get_sample(channel, sample);
            }
        }
    }
}
