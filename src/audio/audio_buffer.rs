use crate::utils::number_array::NumberArray;

#[derive(Debug)]
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

    pub fn new_with_value(channels: usize, buffer_size: usize, value: f32) -> Self {
        AudioBuffer {
            channels,
            buffer_size,
            data: vec![NumberArray::<f32>::new_with_value(buffer_size, value); channels],
        }
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn max(&self) -> f32 {
        let mut max = 0f32;
        for channel in &self.data {
            for sample in channel {
                max = max.max(*sample);
            }
        }
        max
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
        assert!(self.buffer_size == source.buffer_size);

        for channel in 0..self.channels {
            if channel < source.channels() {
                for sample in 0..self.buffer_size {
                    self.data[channel][sample] += source.get_sample(channel, sample);
                }
            }
        }
    }

    pub fn subtract_from(&mut self, source: &AudioBuffer) {
        assert!(self.buffer_size == source.buffer_size);

        for channel in 0..self.channels {
            if channel < source.channels() {
                for sample in 0..self.buffer_size {
                    self.data[channel][sample] -= source.get_sample(channel, sample);
                }
            }
        }
    }

    pub fn multiply_by(&mut self, other: &AudioBuffer) {
        assert!(self.buffer_size == other.buffer_size());

        for channel in 0..self.channels {
            if channel < other.channels() {
                for sample in 0..self.buffer_size {
                    self.data[channel][sample] *= other.get_sample(channel, sample);
                }
            }
        }
    }

    pub fn apply_gain(&mut self, gain: f32) {
        for channel in 0..self.channels {
            for sample in 0..self.buffer_size {
                self.data[channel][sample] *= gain;
            }
        }
    }

    pub fn apply_add(&mut self, add: f32) {
        for channel in 0..self.channels {
            for sample in 0..self.buffer_size {
                self.data[channel][sample] += add;
            }
        }
    }
}
