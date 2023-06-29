use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, BuildStreamError, Device, Sample, StreamConfig, SupportedStreamConfig, FromSample,
};
// use rand::Rng;
use std::{error::Error, fmt};

use crate::runtime::vm::VM;

#[derive(Debug)]
pub struct DeviceError(String);

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Device error: {}", self.0)
    }
}

impl Error for DeviceError {}

#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Config error: {}", self.0)
    }
}

impl Error for ConfigError {}

#[derive(Debug)]
pub struct StreamError(BuildStreamError);

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Stream error: {}", self.0)
    }
}

impl Error for StreamError {}

/// A simple wrapper around `cpal::Stream` to take care of binding callbacks so we can always use f32 in the front-end.
pub struct Stream {
    length: f32,
    config: StreamConfig,
    stream: cpal::Stream,
}

unsafe impl Send for Stream {}

impl Stream {
    pub fn new(vm_ref: &VM) -> Result<Self, Box<dyn Error>> {
        let device = get_device()?;
        let config = get_config(&device)?;
        let channels = config.channels() as usize;
        let err_fn = |err| eprintln!("Stream error: {err}");

        let mut vm = vm_ref.clone();
        vm.add_config(config.clone());
        let length = vm.finalise(config.sample_rate());

        Ok(Stream {
            length,
            config: config.config(),
            stream: match config.sample_format() {
                cpal::SampleFormat::I8 => device.build_output_stream(
                    &config.config(),
                    move |data: &mut [i8], _: &cpal::OutputCallbackInfo| {
                        Self::audio_callback::<i8>(channels, data, &mut vm)
                    },
                    err_fn,
                    None,
                )?,
                cpal::SampleFormat::I16 => device.build_output_stream(
                    &config.config(),
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        Self::audio_callback::<i16>(channels, data, &mut vm)
                    },
                    err_fn,
                    None,
                )?,
                cpal::SampleFormat::I32 => device.build_output_stream(
                    &config.config(),
                    move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                        Self::audio_callback::<i32>(channels, data, &mut vm)
                    },
                    err_fn,
                    None,
                )?,
                cpal::SampleFormat::I64 => device.build_output_stream(
                    &config.config(),
                    move |data: &mut [i64], _: &cpal::OutputCallbackInfo| {
                        Self::audio_callback::<i64>(channels, data, &mut vm)
                    },
                    err_fn,
                    None,
                )?,
                cpal::SampleFormat::U8 => device.build_output_stream(
                    &config.config(),
                    move |data: &mut [u8], _: &cpal::OutputCallbackInfo| {
                        Self::audio_callback::<u8>(channels, data, &mut vm)
                    },
                    err_fn,
                    None,
                )?,
                cpal::SampleFormat::U16 => device.build_output_stream(
                    &config.config(),
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        Self::audio_callback::<u16>(channels, data, &mut vm)
                    },
                    err_fn,
                    None,
                )?,
                cpal::SampleFormat::U32 => device.build_output_stream(
                    &config.config(),
                    move |data: &mut [u32], _: &cpal::OutputCallbackInfo| {
                        Self::audio_callback::<u32>(channels, data, &mut vm)
                    },
                    err_fn,
                    None,
                )?,
                cpal::SampleFormat::U64 => device.build_output_stream(
                    &config.config(),
                    move |data: &mut [u64], _: &cpal::OutputCallbackInfo| {
                        Self::audio_callback::<u64>(channels, data, &mut vm)
                    },
                    err_fn,
                    None,
                )?,
                cpal::SampleFormat::F32 => device.build_output_stream(
                    &config.config(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        Self::audio_callback::<f32>(channels, data, &mut vm)
                    },
                    err_fn,
                    None,
                )?,
                cpal::SampleFormat::F64 => device.build_output_stream(
                    &config.config(),
                    move |data: &mut [f64], _: &cpal::OutputCallbackInfo| {
                        Self::audio_callback::<f64>(channels, data, &mut vm)
                    },
                    err_fn,
                    None,
                )?,
                _ => unreachable!(),
            },
        })
    }

    pub fn length_secs(&self) -> f32 {
        self.length
    }

    pub fn play(&self) -> Result<(), cpal::PlayStreamError> {
        self.stream.play()
    }

    pub fn pause(&self) -> Result<(), cpal::PauseStreamError> {
        self.stream.pause()
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    pub fn buffer_size(&self) -> &BufferSize {
        &self.config.buffer_size
    }

    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    fn audio_callback<T>(channels: usize, data: &mut [T], vm: &mut VM)
    where
        T: FromSample<f32> + Sample,
    {
        let buffer = vm.get_next_buffer(channels, data.len() / channels);

        for (sample_index, frame) in data.chunks_mut(channels).enumerate() {
            for (channel_index, sample) in frame.iter_mut().enumerate() {
                *sample = Sample::from_sample(buffer.get_sample(channel_index, sample_index));
            }
        }
    }
}

fn get_device() -> Result<Device, Box<dyn Error>> {
    let host = cpal::default_host();
    let device = host.default_output_device();
    match device {
        Some(device) => Ok(device),
        None => Err(Box::new(DeviceError(
            "No output device available".to_string(),
        ))),
    }
}

fn get_config(device: &Device) -> Result<SupportedStreamConfig, Box<dyn Error>> {
    let mut configs = device.supported_output_configs()?;
    Ok(configs
        .next()
        .ok_or_else(|| {
            Box::new(ConfigError(
                "No output configurations supported".to_string(),
            ))
        })?
        .with_max_sample_rate())
}
