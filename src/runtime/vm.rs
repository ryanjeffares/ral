use cpal::SupportedStreamConfig;

use crate::{
    audio::{self, audio_buffer::AudioBuffer, components::generators::{noise::Noise, generator::{Generator, StreamInfo}}},
    runtime::instrument::{Instrument, VariableType, InstrumentEventInstance},
    runtime::value::Value,
};
use std::{collections::HashMap, error::Error};

#[derive(Clone)]
pub struct VM {
    instruments: Vec<Instrument>,
    score_events: Vec<ScoreEvent>,
    sorted_score_events: HashMap<usize, Vec<ScoreEvent>>,
    active_score_events: Vec<InstrumentEventInstance>,
    sample_counter: usize,
    audio_config: Option<SupportedStreamConfig>,
}

#[derive(Clone, Debug)]
struct ScoreEvent {
    instrument_index: usize,
    start_time: f32,
    duration: f32,
    init_args: Vec<Value>,
    perf_args: Vec<Value>,
}

unsafe impl Send for VM {}

impl VM {
    pub fn new() -> Self {
        VM {
            instruments: Vec::<Instrument>::new(),
            score_events: Vec::<ScoreEvent>::new(),
            sorted_score_events: HashMap::<usize, Vec<ScoreEvent>>::new(),
            active_score_events: Vec::<InstrumentEventInstance>::new(),
            sample_counter: 0,
            audio_config: None,
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

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let stream = audio::stream::Stream::new(&self)?;
        stream.play()?;
        std::thread::sleep(std::time::Duration::from_millis(3000));
        Ok(())
    }

    pub fn sort_score_events(&mut self, sample_rate: cpal::SampleRate) {
        let sr = sample_rate.0 as f32;
        println!("Sample Rate: {sr}");
        for event in self.score_events.iter() {
            let sample = (event.start_time * sr) as usize;
            println!("Adding sorted score event at {sample}: {event:?}");
            if self.sorted_score_events.contains_key(&sample) {
                self.sorted_score_events
                    .get_mut(&sample)
                    .unwrap()
                    .push(event.clone());
            } else {
                self.sorted_score_events.insert(sample, vec![event.clone()]);
            }
        }
    }

    pub fn get_next_buffer(&mut self, channels: usize, buffer_size: usize) -> AudioBuffer {
        let mut buffer_to_fill = AudioBuffer::new(channels, buffer_size);
        for _ in 0..buffer_size {
            if let Some(events) = self.sorted_score_events.get(&self.sample_counter) {
                for event in events.iter() {
                    let index = event.instrument_index;
                    let mut instrument = self.instruments[index].create_event_instance(
                        event.duration as usize * self.config().sample_rate().0 as usize,
                        event.perf_args.clone(),
                    );
                    instrument.run_init(&event.init_args);
                    self.active_score_events.push(instrument);
                }
            }
            self.sample_counter += 1;
        }

        let mut i = 0;
        while i < self.active_score_events.len() {
            if self.active_score_events[i].run_perf(&mut buffer_to_fill) {
                self.active_score_events.remove(i);
            }
            i += 1;
        }

        let stream_info = StreamInfo {
            sample_rate: self.config().sample_rate(),
            buffer_size,
            channels,
        };

        let mut noise = Noise::new();
        buffer_to_fill.add_from(&noise.get_next_audio_block(&stream_info, vec![Value::float(1.0)]));

        buffer_to_fill
    }
}
