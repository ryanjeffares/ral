use cpal::SupportedStreamConfig;
use phf::phf_map;
use sndfile::{OpenOptions, WriteOptions, SndFileIO};

use crate::{
    audio::{
        self,
        audio_buffer::AudioBuffer,
        components::{
            component::{Component, StreamInfo},
            generators::{
                adsr::Adsr, generator::Generator, mtof::Mtof, noise::Noise, oscil::Oscil,
                padsr::Padsr, sample::Sample,
            },
        },
    },
    runtime::instrument::{Instrument, InstrumentEventInstance, VariableType},
    runtime::value::Value,
};

use std::{
    collections::HashMap,
    error::Error,
    time::{Duration, Instant},
};

static COMPONENTS: phf::Map<&'static str, ComponentInfo> = phf_map! {
    "Noise" => ComponentInfo {
        factory: || Box::new(Noise::new()),
        input_types: &Noise::INPUT_TYPES,
        output_type: Noise::OUTPUT_TYPE,
    },
    "Oscil" => ComponentInfo {
        factory: || Box::new(Oscil::new()),
        input_types: &Oscil::INPUT_TYPES,
        output_type: Oscil::OUTPUT_TYPE,
    },
    "Mtof" => ComponentInfo {
        factory: || Box::new(Mtof{}),
        input_types: &Mtof::INPUT_TYPES,
        output_type: Mtof::OUTPUT_TYPE,
    },
    "Adsr" => ComponentInfo {
        factory: || Box::new(Adsr::new()),
        input_types: &Adsr::INPUT_TYPES,
        output_type: Adsr::OUTPUT_TYPE,
    },
    "Padsr" => ComponentInfo {
        factory: || Box::new(Padsr::new()),
        input_types: &Padsr::INPUT_TYPES,
        output_type: Padsr::OUTPUT_TYPE,
    },
    "WavPlayer" => ComponentInfo {
        factory: || Box::new(Sample::new()),
        input_types: &Sample::INPUT_TYPES,
        output_type: Sample::OUTPUT_TYPE,
    }
};

#[derive(Clone, Copy, PartialEq)]
pub enum OutputTarget {
    Dac,
    File,
    None,
}

#[derive(PartialEq)]
pub enum LogLevel {
    Everything,
    FinalStats,
    Nothing,
}

#[derive(Clone)]
pub struct VM {
    instruments: Vec<Instrument>,
    score_events: Vec<ScoreEvent>,
    sorted_score_events: HashMap<usize, Vec<ScoreEvent>>,
    active_score_events: Vec<InstrumentEventInstance>,
    sample_counter: usize,
    audio_config: Option<SupportedStreamConfig>,
    total_perf_time: Duration,
    max_perf_time: Duration,
    perf_count: u32,
}

unsafe impl Send for VM {}

#[derive(Clone, Debug)]
struct ScoreEvent {
    instrument_index: usize,
    start_time: f32,
    duration: f32,
    init_args: Vec<Value>,
    perf_args: Vec<Value>,
    // these static references are created by a Box::leak call after the whole score is compiled.
    // this is to avoid copying the Vecs for the score event, instead the InstrumentEventInstance
    // can take a static reference.
    // this leaks right now, but maybe that's fine?
    final_init_args: Option<&'static Vec<Value>>,
    final_perf_args: Option<&'static Vec<Value>>,
}

#[derive(Clone)]
pub struct ComponentInfo {
    pub factory: fn() -> Box<dyn Component>,
    pub input_types: &'static [VariableType],
    pub output_type: VariableType,
}

pub fn has_component(component_name: &str) -> bool {
    COMPONENTS.contains_key(component_name)
}

pub fn component_info(component_name: &str) -> ComponentInfo {
    COMPONENTS.get(component_name).unwrap().clone()
}

impl VM {
    pub fn new() -> Self {
        VM {
            instruments: Vec::<Instrument>::new(),
            score_events: Vec::<ScoreEvent>::new(),
            sorted_score_events: HashMap::<usize, Vec<ScoreEvent>>::new(),
            active_score_events: Vec::<InstrumentEventInstance>::new(),
            sample_counter: 0,
            audio_config: None,
            total_perf_time: Duration::ZERO,
            max_perf_time: Duration::ZERO,
            perf_count: 0,
        }
    }

    pub fn add_instrument(&mut self, instrument: Instrument) {
        self.instruments.push(instrument);
    }

    pub fn has_instrument(&self, instrument_name: &String) -> bool {
        self.instruments
            .iter()
            .any(|instrument| instrument.name() == instrument_name)
    }

    pub fn instrument_num_init_args(&self, instrument_name: &String) -> usize {
        self.instruments
            .iter()
            .find(|instrument| instrument.name() == instrument_name)
            .unwrap()
            .num_init_args()
    }

    pub fn instrument_num_perf_args(&self, instrument_name: &String) -> usize {
        self.instruments
            .iter()
            .find(|instrument| instrument.name() == instrument_name)
            .unwrap()
            .num_perf_args()
    }

    pub fn instrument_init_arg_type(&self, instrument_name: &String, index: usize) -> VariableType {
        self.instruments
            .iter()
            .find(|instrument| instrument.name() == instrument_name)
            .unwrap()
            .init_arg_type(index)
    }

    pub fn instrument_perf_arg_type(&self, instrument_name: &String, index: usize) -> VariableType {
        self.instruments
            .iter()
            .find(|instrument| instrument.name() == instrument_name)
            .unwrap()
            .perf_arg_type(index)
    }

    pub fn add_score_event(
        &mut self,
        instrument_name: &String,
        start_time: f32,
        duration: f32,
        init_args: Vec<Value>,
        perf_args: Vec<Value>,
    ) {
        let instrument_index = self
            .instruments
            .iter()
            .position(|instrument| instrument.name() == instrument_name)
            .unwrap();

        self.score_events.push(ScoreEvent {
            instrument_index,
            start_time,
            duration,
            init_args,
            perf_args,
            final_init_args: None,
            final_perf_args: None,
        });
    }

    pub fn print_ops(&self) {
        for instrument in &self.instruments {
            instrument.print_ops();
        }
    }

    pub fn add_config(&mut self, config: SupportedStreamConfig) {
        self.audio_config = Some(config);
    }

    pub fn config(&self) -> &SupportedStreamConfig {
        self.audio_config.as_ref().unwrap()
    }

    pub fn run(&mut self, output_target: OutputTarget) -> Result<(), Box<dyn Error>> {
        match output_target {
            OutputTarget::Dac => {
                let stream = audio::stream::Stream::new(self)?;
                println!("Opened stream, Sample Rate: {}", stream.sample_rate());
                stream.play()?;

                // TODO: hacky way to make sure init calls still happen on 0 length scores
                std::thread::sleep(std::time::Duration::from_secs_f32(
                    stream.length_secs().max(0.1),
                ));

                println!(
                    "Real time performance finished in {}s",
                    stream.length_secs()
                );

                Ok(())
            }
            OutputTarget::File => self.write_to_file(),
            OutputTarget::None => self.run_no_output(),
        }
    }

    pub fn finalise(&mut self, sample_rate: cpal::SampleRate) -> f32 {
        for instrument in self.instruments.iter_mut() {
            instrument.finalise();
        }

        let sr = sample_rate.0 as f32;
        let mut last_end_sample = 0.0;
        for event in self.score_events.iter_mut() {
            event.final_init_args = Some(Box::leak(Box::new(event.init_args.clone())));
            event.final_perf_args = Some(Box::leak(Box::new(event.perf_args.clone())));

            let sample = (event.start_time * sr) as usize;
            let end_time = event.start_time + event.duration;
            if end_time > last_end_sample {
                last_end_sample = end_time;
            }
            // println!("Adding sorted score event at {sample}: {event:?}");
            if self.sorted_score_events.contains_key(&sample) {
                self.sorted_score_events
                    .get_mut(&sample)
                    .unwrap()
                    .push(event.clone());
            } else {
                self.sorted_score_events.insert(sample, vec![event.clone()]);
            }
        }

        // println!("{:?}", self.sorted_score_events);
        last_end_sample
    }

    pub fn get_next_buffer(&mut self, channels: usize, buffer_size: usize) -> AudioBuffer {
        // let _timer = Timer::new("VM::get_next_buffer()");
        let timer = Instant::now();

        let mut buffer_to_fill = AudioBuffer::new(channels, buffer_size);
        let stream_info = StreamInfo {
            sample_rate: self.config().sample_rate().0,
            buffer_size,
            channels,
        };

        for _ in 0..buffer_size {
            if let Some(events) = self.sorted_score_events.get(&self.sample_counter) {
                for event in events.iter() {
                    let index = event.instrument_index;
                    let mut instrument = self.instruments[index].create_event_instance(
                        (event.duration * self.config().sample_rate().0 as f32) as usize,
                        event.final_init_args.unwrap(),
                        event.final_perf_args.unwrap(),
                    );
                    instrument.run_init(&stream_info, &mut buffer_to_fill);
                    self.active_score_events.push(instrument);
                }
            }
            self.sample_counter += 1;
        }

        // TODO: instrument execution order
        let mut i = 0;
        while i < self.active_score_events.len() {
            if self.active_score_events[i].run_perf(&stream_info, &mut buffer_to_fill) {
                self.active_score_events.remove(i);
            } else {
                i += 1;
            }
        }

        // println!("Max amplitude of buffer: {}", buffer_to_fill.max());
        let time = timer.elapsed();
        // println!("Perf completed in {time:?}");
        self.max_perf_time = self.max_perf_time.max(time);
        self.total_perf_time += time;
        self.perf_count += 1;
        buffer_to_fill
    }

    fn write_to_file(&mut self) -> Result<(), Box<dyn Error>> {
        const SAMPLE_RATE: u32 = 48000;
        const BUFFER_SIZE: u32 = SAMPLE_RATE / 100;
        const CHANNELS: u16 = 2;

        self.add_config(SupportedStreamConfig::new(
            CHANNELS,
            cpal::SampleRate(SAMPLE_RATE),
            cpal::SupportedBufferSize::Range {
                min: BUFFER_SIZE,
                max: BUFFER_SIZE,
            },
            cpal::SampleFormat::F32,
        ));

        let len = (self.finalise(self.config().sample_rate()) * (SAMPLE_RATE as f32)) as usize;
        let path = std::env::current_dir()?.join("test.wav");
        let mut snd = match OpenOptions::WriteOnly(WriteOptions::new(sndfile::MajorFormat::WAV, sndfile::SubtypeFormat::FLOAT, sndfile::Endian::CPU, 48000, 2)).from_path(path) {
            Ok(snd) => snd,
            Err(err) => {
                panic!("Failed to open file: {:?}", err);
            }
        };

        let mut sample_counter = 0;
        let mut samples = Vec::<f32>::new();
        while sample_counter < len {
            let buff = self.get_next_buffer(CHANNELS as usize, BUFFER_SIZE as usize);
            for sample in 0..buff.buffer_size() {
                for channel in 0..buff.channels() {
                    samples.push(buff.get_sample(channel, sample));
                }
            }
            sample_counter += 480;
        }

        match snd.write_from_slice(samples.as_slice()) {
            Ok(len) => println!("{len} samples written to test.wav"),
            Err(err) => eprintln!("Failed to write to wav: {:?}", err),
        }
        
        Ok(())
    }

    fn run_no_output(&mut self) -> Result<(), Box<dyn Error>> {
        const SAMPLE_RATE: u32 = 48000;
        const BUFFER_SIZE: u32 = SAMPLE_RATE / 100;
        const CHANNELS: u16 = 2;

        self.add_config(SupportedStreamConfig::new(
            CHANNELS,
            cpal::SampleRate(SAMPLE_RATE),
            cpal::SupportedBufferSize::Range {
                min: BUFFER_SIZE,
                max: BUFFER_SIZE,
            },
            cpal::SampleFormat::F32,
        ));

        let len = (self.finalise(self.config().sample_rate()) * (SAMPLE_RATE as f32)) as usize;

        let mut sample_counter = 0;
        while sample_counter < len {
            self.get_next_buffer(CHANNELS as usize, BUFFER_SIZE as usize);
            sample_counter += 480;
        }

        Ok(())
    }
}

impl Drop for VM {
    fn drop(&mut self) {
        if self.perf_count > 0 {
            println!("Max perf time: {:?}", self.max_perf_time);
            println!(
                "Average perf time: {:?}",
                self.total_perf_time / self.perf_count
            );
        }
    }
}
