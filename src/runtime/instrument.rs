use std::fmt;

use crate::{
    audio::audio_buffer::AudioBuffer, runtime::ops::Op, runtime::value::Value,
    utils::number_array::NumberArray,
};

#[derive(Clone)]
struct Function {
    ops: Vec<Op>,
    constants: Vec<Value>,
    args: Vec<InstrumentVariable>,
    locals: Vec<InstrumentVariable>,
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
    init_func: Function,    
    init_args: Vec<Value>,
    perf_func: Function,
    perf_args: Vec<Value>,
    duration_samples: usize,
    sample_counter: usize,
}

#[derive(Clone, Debug)]
struct InstrumentVariable {
    variable_index: usize,
    variable_name: String,
    variable_type: VariableType,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum VariableType {
    Audio,
    Float,
    Int,
    None,
    String,
}

impl Function {
    fn new() -> Self {
        Function {
            ops: Vec::<Op>::new(),
            constants: Vec::<Value>::new(),
            args: Vec::<InstrumentVariable>::new(),
            locals: Vec::<InstrumentVariable>::new(),
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

    pub fn create_event_instance(
        &self,
        duration_samples: usize,
        init_args: Vec<Value>,
        perf_args: Vec<Value>,
    ) -> InstrumentEventInstance {
        InstrumentEventInstance {
            instrument_name: self.instrument_name.clone(),
            variables: vec![Value::default(); self.variables.len()],
            init_func: self.init_func.clone(),
            init_args,
            perf_func: self.perf_func.clone(),
            perf_args,
            duration_samples,
            sample_counter: 0,
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
        self.variables.push(InstrumentVariable::new(
            self.variables.len(),
            variable_name,
            variable_type,
        ));
    }

    pub fn add_init_local(&mut self, variable_name: String, variable_type: VariableType) -> bool {
        if self.get_init_arg(&variable_name).is_some()
            || self.get_variable(&variable_name).is_some()
            || self.get_local_init_variable(&variable_name).is_some()
        {
            false
        } else {
            self.init_func.locals.push(InstrumentVariable::new(
                self.init_func.locals.len(),
                variable_name,
                variable_type,
            ));
            true
        }
    }

    pub fn add_perf_local(&mut self, variable_name: String, variable_type: VariableType) {
        self.perf_func.locals.push(InstrumentVariable::new(
            self.perf_func.locals.len(),
            variable_name,
            variable_type,
        ));
    }

    pub fn add_init_arg(
        &mut self,
        arg_index: usize,
        arg_name: String,
        arg_type: VariableType,
    ) -> bool {
        if self.get_init_arg(&arg_name).is_some() {
            false
        } else if self.get_variable(&arg_name).is_some() {
            false
        } else {
            self.init_func
                .args
                .push(InstrumentVariable::new(arg_index, arg_name, arg_type));
            true
        }
    }

    pub fn add_perf_arg(
        &mut self,
        arg_index: usize,
        arg_name: String,
        arg_type: VariableType,
    ) -> bool {
        if self.get_perf_arg(&arg_name).is_some() {
            false
        } else if self.get_variable(&arg_name).is_some() {
            false
        } else {
            self.perf_func
                .args
                .push(InstrumentVariable::new(arg_index, arg_name, arg_type));
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

    pub fn emit_init_op(&mut self, op: Op) {
        self.init_func.ops.push(op);
    }

    pub fn emit_init_constant(&mut self, value: Value) {
        self.init_func.constants.push(value);
    }

    pub fn emit_perf_op(&mut self, op: Op) {
        self.perf_func.ops.push(op);
    }

    pub fn emit_perf_constant(&mut self, value: Value) {
        self.perf_func.constants.push(value);
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
    pub fn create_default_args(&self, perf: bool) -> Vec<Value> {
        if perf {
            &self.perf_func
        } else {
            &self.init_func
        }
        .args
        .iter()
        .map(|arg| match arg.variable_type {
            VariableType::Audio => Value::audio(NumberArray::<f32>::new(32)),
            VariableType::Float => Value::float(0.0),
            VariableType::Int => Value::int(0),
            VariableType::String => Value::string(String::new()),
            _ => unreachable!(),
        })
        .collect()
    }

    pub fn run_init(&mut self) {
        println!("INFO: running init for {}", self.instrument_name);
        self.run_ops(false);
    }

    /// Returns true when the event is over
    #[must_use]
    pub fn run_perf(&mut self, buffer_to_fill: &mut AudioBuffer) -> bool {
        self.run_ops(true);
        self.sample_counter += buffer_to_fill.buffer_size();
        self.sample_counter >= self.duration_samples
    }

    fn run_ops(&mut self, perf: bool) {
        let func = if perf {
            &self.perf_func
        } else {
            &self.init_func
        };

        let args = if perf {
            &self.init_args
        } else {
            &self.perf_args
        };

        let mut stack = Vec::<Value>::new();
        let mut locals = Vec::<Value>::new();
        let mut constant_idx = 0;

        for op in &func.ops {
            match op {
                Op::AssignLocal => {
                    let index = func.constants[constant_idx].get_int() as usize;
                    constant_idx += 1;
                    locals[index] = stack.pop().unwrap();
                }
                Op::AssignMember => {
                    let index = func.constants[constant_idx].get_int() as usize;
                    constant_idx += 1;
                    self.variables[index] = stack.pop().unwrap();
                }
                Op::DeclareLocal => {
                    let value = stack.pop().unwrap();
                    locals.push(value);
                }
                Op::LoadArg => {
                    let index = func.constants[constant_idx].get_int() as usize;
                    constant_idx += 1;
                    stack.push(args[index].clone());
                }
                Op::LoadConstant => {
                    stack.push(func.constants[constant_idx].clone());
                    constant_idx += 1;
                }
                Op::LoadLocal => {
                    let index = func.constants[constant_idx].get_int() as usize;
                    constant_idx += 1;
                    stack.push(locals[index].clone());
                }
                Op::LoadMember => {
                    let index = func.constants[constant_idx].get_int() as usize;
                    constant_idx += 1;
                    stack.push(self.variables[index].clone());
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
            }
        }
    }
}

impl Drop for InstrumentEventInstance {
    fn drop(&mut self) {
        println!("INFO: score event ended for {}", self.instrument_name);
    }
}
impl InstrumentVariable {
    pub fn new(variable_index: usize, variable_name: String, variable_type: VariableType) -> Self {
        InstrumentVariable {
            variable_index,
            variable_name,
            variable_type,
        }
    }

    pub fn name(&self) -> &String {
        &self.variable_name
    }
}