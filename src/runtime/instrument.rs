use std::fmt;

use crate::{
    audio::{
        audio_buffer::AudioBuffer,
        components::component::{Component, ComponentType, StreamInfo},
    },
    runtime::ops::Op,
    runtime::value::Value,
};

#[derive(Clone)]
struct Function {
    ops: Vec<Op>,
    final_ops: Option<&'static Vec<Op>>,
    args: Vec<InstrumentVariable>,
    locals: Vec<InstrumentVariable>,
    components: Vec<Box<dyn Component>>,
}

#[derive(Clone)]
struct FunctionEventInstance {
    // this static reference is created by a Box::leak call in the VM after the whole score is compiled.
    // this is to avoid copying the Vec from the function for each score event, instead we can take a static reference here.
    // this leaks right now, but maybe that's fine?
    ops: &'static Vec<Op>,
    components: Vec<Box<dyn Component>>,
}

#[derive(Clone)]
pub struct Instrument {
    instrument_name: String,
    variables: Vec<InstrumentVariable>,
    init_func: Function,
    perf_func: Function,
}

#[derive(Clone)]
pub struct InstrumentEventInstance {
    instrument_name: String,
    variables: Vec<Value>,
    init_func: FunctionEventInstance,
    perf_func: FunctionEventInstance,
    // these static references are created by a Box::leak call in the VM after the whole score is compiled.
    // this is to avoid copying the Vecs from the score event, instead we can take a static reference here.
    // this leaks right now, but maybe that's fine?
    init_args: &'static Vec<Value>,
    perf_args: &'static Vec<Value>,
    duration_samples: usize,
    sample_counter: usize,
    max_amps: f32,
}

#[derive(Clone, Debug)]
struct InstrumentVariable {
    variable_name: String,
    variable_type: VariableType,
}

impl VariableType {
    pub fn can_factor_with(&self, other: VariableType) -> bool {
        match self {
            VariableType::Audio => other != VariableType::String,
            VariableType::Float => other == VariableType::Float || other == VariableType::Int,
            VariableType::Int => other == VariableType::Float || other == VariableType::Int,
            VariableType::String => false,
        }
    }

    pub fn can_sum_with(&self, other: VariableType) -> bool {
        match self {
            VariableType::Audio => other != VariableType::String,
            VariableType::Float => other == VariableType::Float || other == VariableType::Int,
            VariableType::Int => other == VariableType::Float || other == VariableType::Int,
            VariableType::String => other == VariableType::String,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum VariableType {
    Audio,
    Float,
    Int,
    String,
}

impl Function {
    fn new() -> Self {
        Function {
            ops: Vec::<Op>::new(),
            final_ops: None,
            args: Vec::<InstrumentVariable>::new(),
            locals: Vec::<InstrumentVariable>::new(),
            components: Vec::<Box<dyn Component>>::new(),
        }
    }

    fn finalise(&mut self) {
        self.final_ops = Some(Box::leak(Box::new(self.ops.clone())));
    }

    fn create_event_instance(&self) -> FunctionEventInstance {
        FunctionEventInstance {
            ops: self.final_ops.unwrap(),
            components: self.components.clone(),
        }
    }
}

impl Instrument {
    pub fn new(instrument_name: String) -> Self {
        Instrument {
            instrument_name,
            variables: Vec::<InstrumentVariable>::new(),
            init_func: Function::new(),
            perf_func: Function::new(),
        }
    }

    pub fn finalise(&mut self) {
        self.init_func.finalise();
        self.perf_func.finalise();
    }

    pub fn create_event_instance(
        &self,
        duration_samples: usize,
        init_args: &'static Vec<Value>,
        perf_args: &'static Vec<Value>,
    ) -> InstrumentEventInstance {
        InstrumentEventInstance {
            instrument_name: self.instrument_name.clone(),
            variables: vec![Value::default(); self.variables.len()],
            init_func: self.init_func.create_event_instance(),
            perf_func: self.perf_func.create_event_instance(),
            init_args,
            perf_args,
            duration_samples,
            sample_counter: 0,
            max_amps: 0.0,
        }
    }

    pub fn name(&self) -> &String {
        &self.instrument_name
    }

    pub fn print_ops(&self) {
        println!("{}:", self.instrument_name);
        println!("\tinit:");

        for op in &self.init_func.ops {
            println!("\t\t{op:?}");
        }

        println!("\tperf:");
        for op in &self.perf_func.ops {
            println!("\t\t{op:?}");
        }
    }

    pub fn num_init_args(&self) -> usize {
        self.init_func.args.len()
    }

    pub fn init_arg_type(&self, index: usize) -> VariableType {
        self.init_func.args[index].variable_type
    }

    pub fn num_perf_args(&self) -> usize {
        self.perf_func.args.len()
    }

    pub fn perf_arg_type(&self, index: usize) -> VariableType {
        self.perf_func.args[index].variable_type
    }

    pub fn add_variable(&mut self, variable_name: String, variable_type: VariableType) {
        self.variables
            .push(InstrumentVariable::new(variable_name, variable_type));
    }

    pub fn add_init_local(&mut self, variable_name: String, variable_type: VariableType) -> bool {
        if self.get_init_arg(&variable_name).is_some()
            || self.get_variable(&variable_name).is_some()
            || self.get_local_init_variable(&variable_name).is_some()
        {
            false
        } else {
            self.init_func
                .locals
                .push(InstrumentVariable::new(variable_name, variable_type));
            true
        }
    }

    pub fn add_perf_local(&mut self, variable_name: String, variable_type: VariableType) -> bool {
        if self.get_perf_arg(&variable_name).is_some()
            || self.get_variable(&variable_name).is_some()
            || self.get_local_perf_variable(&variable_name).is_some()
        {
            false
        } else {
            self.perf_func
                .locals
                .push(InstrumentVariable::new(variable_name, variable_type));
            true
        }
    }

    pub fn add_init_arg(&mut self, arg_name: String, arg_type: VariableType) -> bool {
        if self.get_init_arg(&arg_name).is_some() || self.get_variable(&arg_name).is_some() {
            false
        } else {
            self.init_func
                .args
                .push(InstrumentVariable::new(arg_name, arg_type));
            true
        }
    }

    pub fn add_perf_arg(&mut self, arg_name: String, arg_type: VariableType) -> bool {
        if self.get_perf_arg(&arg_name).is_some() || self.get_variable(&arg_name).is_some() {
            false
        } else {
            self.perf_func
                .args
                .push(InstrumentVariable::new(arg_name, arg_type));
            true
        }
    }

    pub fn get_variable(&self, variable_name: &String) -> Option<usize> {
        self.variables
            .iter()
            .position(|variable| &variable.variable_name == variable_name)
    }

    pub fn member_type(&self, index: usize) -> VariableType {
        self.variables[index].variable_type
    }

    pub fn init_local_type(&self, index: usize) -> VariableType {
        self.init_func.locals[index].variable_type
    }

    pub fn perf_local_type(&self, index: usize) -> VariableType {
        self.perf_func.locals[index].variable_type
    }

    pub fn get_local_init_variable(&self, variable_name: &String) -> Option<usize> {
        self.init_func
            .locals
            .iter()
            .position(|variable| &variable.variable_name == variable_name)
    }

    pub fn get_local_perf_variable(&self, variable_name: &String) -> Option<usize> {
        self.perf_func
            .locals
            .iter()
            .position(|variable| &variable.variable_name == variable_name)
    }

    pub fn get_init_arg(&self, arg_name: &String) -> Option<usize> {
        self.init_func
            .args
            .iter()
            .position(|arg| &arg.variable_name == arg_name)
    }

    pub fn get_perf_arg(&self, arg_name: &String) -> Option<usize> {
        self.perf_func
            .args
            .iter()
            .position(|arg| &arg.variable_name == arg_name)
    }

    pub fn add_init_component(&mut self, component: Box<dyn Component>) -> usize {
        self.init_func.components.push(component);
        self.init_func.components.len() - 1
    }

    pub fn add_perf_component(&mut self, component: Box<dyn Component>) -> usize {
        self.perf_func.components.push(component);
        self.perf_func.components.len() - 1
    }

    pub fn emit_init_op(&mut self, op: Op) {
        self.init_func.ops.push(op);
    }

    pub fn emit_perf_op(&mut self, op: Op) {
        self.perf_func.ops.push(op);
    }
}

impl fmt::Display for Instrument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Instrument {{ name: {}, variables: {:?} }}",
            self.instrument_name, self.variables
        )
    }
}

impl InstrumentEventInstance {
    pub fn run_init(&mut self, stream_info: &StreamInfo, buffer_to_fill: &mut AudioBuffer) {
        println!("INFO: running init for {}", self.instrument_name);
        self.run_ops(false, stream_info, buffer_to_fill);
    }

    /// Returns true when the event is over
    #[must_use]
    pub fn run_perf(&mut self, stream_info: &StreamInfo, buffer_to_fill: &mut AudioBuffer) -> bool {
        // let _timer = Timer::new("Perf func");
        self.run_ops(true, stream_info, buffer_to_fill);
        self.sample_counter += stream_info.buffer_size;
        self.sample_counter >= self.duration_samples
    }

    fn run_ops(&mut self, perf: bool, stream_info: &StreamInfo, buffer_to_fill: &mut AudioBuffer) {
        let func = if perf {
            &mut self.perf_func
        } else {
            &mut self.init_func
        };

        let args = if perf { self.perf_args } else { self.init_args };

        let mut stack = Vec::<Value>::new();
        let mut locals = Vec::<Value>::new();

        for op in func.ops {
            match op {
                Op::AssignLocal(index) => {
                    locals[*index] = stack.pop().unwrap();
                }
                Op::AssignMember(index) => {
                    self.variables[*index] = stack.pop().unwrap();
                }
                Op::CallComponent(index) => {
                    let arg_count = func.components[*index].arg_count();
                    let component_type = func.components[*index].component_type();
                    let mut args = vec![Value::default(); arg_count];
                    for i in 0..arg_count {
                        args[arg_count - i - 1] = stack.pop().unwrap();
                    }

                    match component_type {
                        ComponentType::Generator => {
                            stack.push(func.components[*index].process(stream_info, args));
                        }
                    }
                }
                Op::DeclareLocal => {
                    let value = stack.pop().unwrap();
                    locals.push(value);
                }
                Op::LoadArg(index) => {
                    stack.push(args[*index].clone());
                }
                Op::LoadConstant(value) => {
                    stack.push(value.clone());
                }
                Op::LoadLocal(index) => {
                    stack.push(locals[*index].clone());
                }
                Op::LoadMember(index) => {
                    stack.push(self.variables[*index].clone());
                }
                Op::Output => {
                    let audio = stack.pop().unwrap();
                    self.max_amps = self.max_amps.max(audio.get_audio().max());
                    buffer_to_fill.add_from(audio.get_audio());
                }
                Op::Print => {
                    let value = stack.pop().unwrap();
                    print!("{value}");
                }
                Op::PrintEmpty => {
                    print!("\t");
                }
                Op::PrintLn => {
                    let value = stack.pop().unwrap();
                    println!("{value}");
                }
                Op::PrintLnEmpty => {
                    println!();
                }
                Op::Add => {
                    let rhs = stack.pop().unwrap();
                    let lhs = stack.pop().unwrap();
                    stack.push(lhs + rhs);
                }
                Op::Divide => {
                    let rhs = stack.pop().unwrap();
                    let lhs = stack.pop().unwrap();
                    stack.push(lhs / rhs);
                }
                Op::Multiply => {
                    let rhs = stack.pop().unwrap();
                    let lhs = stack.pop().unwrap();
                    stack.push(lhs * rhs);
                }
                Op::Subtract => {
                    let rhs = stack.pop().unwrap();
                    let lhs = stack.pop().unwrap();
                    stack.push(lhs - rhs);
                }
            }
        }
    }
}

impl Drop for InstrumentEventInstance {
    fn drop(&mut self) {
        // println!(
        //     "INFO: score event ended for {}, max amps: {}",
        //     self.instrument_name, self.max_amps
        // );
    }
}

impl InstrumentVariable {
    pub fn new(variable_name: String, variable_type: VariableType) -> Self {
        InstrumentVariable {
            variable_name,
            variable_type,
        }
    }
}
