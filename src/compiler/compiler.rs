use std::{error::Error, fmt};

use colored::Colorize;

use crate::{
    compiler::scanner::{Scanner, Token, TokenType},
    runtime::instrument::{Instrument, VariableType},
    runtime::ops::Op,
    runtime::value::Value,
    runtime::vm::VM,
    utils::timer::Timer,
};

// Signifies an internal error as opposed to user code error
#[derive(Debug)]
pub struct InternalCompilerError(String);

impl fmt::Display for InternalCompilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Compiler error: {}", self.0)
    }
}

impl Error for InternalCompilerError {}

struct ParseStringError(String);

impl fmt::Display for ParseStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum CompilerContext {
    InitFunc,
    Instrument,
    InstrumentsBlock,
    PerfFunc,
    ScoreBlock,
    TopLevel,
}

struct Compiler {
    file_path: String,
    scanner: Scanner,
    previous: Option<Token>,
    current: Option<Token>,
    had_error: bool,
    context_stack: Vec<CompilerContext>,
    vm: VM,
}

pub fn compile_and_run(code: String, file_path: String) -> Result<(), Box<dyn Error>> {
    let mut compiler = Compiler {
        file_path,
        scanner: Scanner::new(code),
        had_error: false,
        previous: None,
        current: None,
        context_stack: Vec::<CompilerContext>::new(),
        vm: VM::new(),
    };

    {
        let _timer = Timer::new("Compilation");
        compiler.compile();
    }

    if compiler.had_error() {
        eprintln!("Stopping execution due to compilation errors");
    } else {        
        compiler.print_ops();
        let _timer = Timer::new("Run");
        compiler.run()?;
    }

    Ok(())
}

impl Compiler {
    fn compile(&mut self) {
        self.context_stack.push(CompilerContext::TopLevel);
        self.advance();
        loop {
            if self.current.is_some() {
                if self.match_token(TokenType::InstrumentsIdent) {
                    self.instruments_block();
                } else if self.match_token(TokenType::ScoreIdent) {
                    self.score_block();
                } else if self.match_token(TokenType::EndOfFile) {
                    break;
                } else {
                    self.error_at_current(
                        "Invalid token at top level; expected 'instruments' or 'score'".to_string(),
                    );
                    break;
                }
            } else {
                unreachable!();
            }
        }
    }

    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.vm.run()
    }

    fn print_ops(&mut self) {
        self.vm.print_ops();
    }

    fn emit_constant(&mut self, instrument: &mut Instrument, value: Value) {
        match self.context_stack.last().unwrap() {
            CompilerContext::InitFunc => instrument.emit_init_constant(value),
            CompilerContext::PerfFunc => instrument.emit_perf_constant(value),
            _ => unreachable!(),
        }
    }

    fn emit_op(&mut self, instrument: &mut Instrument, op: Op) {
        match self.context_stack.last().unwrap() {
            CompilerContext::InitFunc => instrument.emit_init_op(op),
            CompilerContext::PerfFunc => instrument.emit_perf_op(op),
            _ => unreachable!(),
        }
    }

    fn advance(&mut self) {
        self.previous = self.current.clone();
        self.current = Some(self.scanner.scan_token());
        // println!("{}", self.current.as_ref().unwrap());
    }

    fn match_token(&mut self, expected: TokenType) -> bool {
        if self.check_token(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check_token(&self, expected: TokenType) -> bool {
        self.current.is_some() && self.current.as_ref().unwrap().token_type() == expected
    }

    fn consume(&mut self, expected: TokenType, message: &'static str) {
        if self.check_token(expected) {
            self.advance();
        } else {
            self.error_at_current(message.to_string());
        }
    }

    fn instruments_block(&mut self) {
        self.context_stack.push(CompilerContext::InstrumentsBlock);
        self.consume(TokenType::BraceOpen, "Expected '{'");

        loop {
            if self.match_token(TokenType::Identifier) {
                self.instrument();
            } else if self.match_token(TokenType::BraceClose) {
                break;
            } else {
                self.error_at_current("Invalid token; expected instrument name".to_string());
                return;
            }
        }

        self.context_stack.pop();
    }

    fn instrument(&mut self) {
        self.context_stack.push(CompilerContext::Instrument);
        let mut instrument = Instrument::new(self.previous.as_ref().unwrap().text().clone());
        self.consume(TokenType::BraceOpen, "Expected '{'");

        loop {
            if self.match_token(TokenType::Identifier) {
                self.member_variable(&mut instrument);
            } else if self.match_token(TokenType::InitIdent) {
                self.context_stack.push(CompilerContext::InitFunc);
                self.function(&mut instrument);
                self.context_stack.pop();
            } else if self.match_token(TokenType::PerfIdent) {
                self.context_stack.push(CompilerContext::PerfFunc);
                self.function(&mut instrument);
                self.context_stack.pop();
            } else if self.match_token(TokenType::BraceClose) {
                break;
            } else {
                self.error_at_current("Expected member variable, 'init', or 'perf'".to_string());
                return;
            }
        }

        self.vm.add_instrument(instrument);
        self.context_stack.pop();
    }

    fn member_variable(&mut self, instrument: &mut Instrument) {
        let variable_name = self.previous.as_ref().clone().unwrap().text().clone();
        if variable_name.chars().next().unwrap().is_uppercase() {
            self.error_at_previous("Argument and local identifier names must not begin with a capital letter".to_string());
            return;
        }

        if instrument.get_variable(&variable_name).is_some() {
            self.error_at_previous("Duplicate instrument variable name".to_string());
            return;
        }

        self.consume(TokenType::Colon, "Expected ':'");
        let type_token = self.current.as_ref().unwrap().token_type();
        if type_token.is_type_ident() {
            instrument.add_variable(variable_name, type_token.to_variable_type());
            self.advance(); // consume type ident
            self.consume(TokenType::Semicolon, "Expected ';'");
        } else {
            self.error_at_current("Expected type identifier".to_string());
        }
    }

    fn function(&mut self, instrument: &mut Instrument) {
        let context = *self.context_stack.last().unwrap();

        if self.match_token(TokenType::ParenOpen) {
            let mut arg_count = 0;
            loop {
                if self.match_token(TokenType::Identifier) {
                    let arg_name_token = self.previous.as_ref().unwrap().clone();
                    if arg_name_token.text().chars().next().unwrap().is_uppercase() {
                        self.error_at_previous("Argument and local identifier names must not begin with a capital letter".to_string());
                        return;
                    }

                    self.consume(TokenType::Colon, "Expected ':'");
                    let type_token = self.current.as_ref().unwrap().token_type();
                    if type_token.is_type_ident() {
                        if type_token == TokenType::AudioIdent {
                            self.error_at_current(format!("Invalid type for function argument"));
                            return;
                        }
                        self.advance(); // consume type ident
                        match context {
                            CompilerContext::InitFunc => {
                                if !instrument.add_init_arg(
                                    arg_count,
                                    arg_name_token.text().clone(),
                                    type_token.to_variable_type(),
                                ) {
                                    self.error(
                                        &arg_name_token,
                                        "An argument or member variable with the same name already exists"
                                            .to_string(),
                                    );
                                    return;
                                }
                            }
                            CompilerContext::PerfFunc => {
                                if !instrument.add_perf_arg(
                                    arg_count,
                                    arg_name_token.text().clone(),
                                    type_token.to_variable_type(),
                                ) {
                                    self.error(
                                        &arg_name_token,
                                        "An argument or member variable with the same name already exists"
                                            .to_string(),
                                    );
                                    return;
                                }
                            }
                            _ => unreachable!(),
                        }
                        arg_count += 1;
                    } else {
                        self.error_at_current("Expected type identifier".to_string());
                        return;
                    }

                    if !self.check_token(TokenType::ParenClose) {
                        if !self.match_token(TokenType::Comma) {
                            self.error_at_current("Expected ','".to_string());
                            return;
                        }
                    }
                } else if self.match_token(TokenType::ParenClose) {
                    break;
                } else {
                    self.error_at_current("Expected argument name or ')'".to_string());
                    return;
                }
            }
        }

        self.consume(TokenType::BraceOpen, "Expected '{");

        loop {
            if self.match_token(TokenType::Local) {
                self.local_declaration(instrument);
            } else if self.match_token(TokenType::BraceClose) {
                break;
            } else {
                self.statement(instrument);
            }
        }
    }

    fn local_declaration(&mut self, instrument: &mut Instrument) {
        self.consume(TokenType::Identifier, "Expected identifier");
        let local_name_token = self.previous.as_ref().unwrap().clone();
        if local_name_token.text().chars().next().unwrap().is_uppercase() {
            self.error_at_previous("Argument and local identifier names must not begin with a capital letter".to_string());
            return;
        }
        self.consume(TokenType::Colon, "Expected ':'");

        let type_token = self.current.as_ref().unwrap().token_type();
        if type_token.is_type_ident() {
            if !instrument.add_init_local(
                local_name_token.text().clone(),
                type_token.to_variable_type(),
            ) {
                self.error(&local_name_token, "A member variable, argument, or local variable with the same name already exists".to_string());
                return;
            }
        } else {
            self.error_at_current("Expected type identifier".to_string());
            return;
        }

        self.advance(); // consume type token
        self.consume(TokenType::Equal, "Expected '='");
        let expression_type = self.expression(instrument);
        if expression_type != type_token.to_variable_type() {
            self.error_at_previous(format!("Type mismatch: expected '{:?}' for assignment to local '{}' but got '{expression_type:?}'", type_token.to_variable_type(), local_name_token.text()));
            return;
        }

        self.emit_op(instrument, Op::DeclareLocal);
        self.consume(TokenType::Semicolon, "Expected ';'");
    }

    fn statement(&mut self, instrument: &mut Instrument) {
        if self.match_token(TokenType::Print) {
            self.consume(TokenType::ParenOpen, "Expected '('");
            if self.match_token(TokenType::ParenClose) {
                self.emit_op(instrument, Op::PrintEmpty);
            } else {
                self.expression(instrument);
                self.emit_op(instrument, Op::Print);
                self.consume(TokenType::ParenClose, "Expected ')'");
            }
        } else if self.match_token(TokenType::PrintLn) {
            self.consume(TokenType::ParenOpen, "Expected '('");
            if self.match_token(TokenType::ParenClose) {
                self.emit_op(instrument, Op::PrintLnEmpty);
            } else {
                self.expression(instrument);
                self.emit_op(instrument, Op::PrintLn);
                self.consume(TokenType::ParenClose, "Expected ')'");
            }
        } else if self.match_token(TokenType::Identifier) {
            let variable_name = self.previous.as_ref().unwrap().text().clone();
            if let Some(index) = instrument.get_variable(&variable_name) {
                self.consume(TokenType::Equal, "Expected '='");
                let variable_type = instrument.member_type(index);
                let expression_type = self.expression(instrument);
                if variable_type != expression_type {
                    self.error_at_previous(format!("Expected {variable_type:?} to assign to member variable '{variable_name}' but got {expression_type:?}"));
                    return;
                }
                self.emit_constant(instrument, Value::int(index as i64));
                self.emit_op(instrument, Op::AssignMember);
            } else {
                match self.context_stack.last().unwrap() {
                    CompilerContext::InitFunc => {
                        if let Some(index) = instrument.get_local_init_variable(&variable_name) {
                            self.consume(TokenType::Equal, "Expected '='");
                            let variable_type = instrument.init_local_type(index);
                            let expression_type = self.expression(instrument);
                            if variable_type != expression_type {
                                self.error_at_previous(format!("Expected {variable_type:?} to assign to local variable '{variable_name}' but got {expression_type:?}"));
                                return;
                            }
                            self.emit_constant(instrument, Value::int(index as i64));
                            self.emit_op(instrument, Op::AssignLocal);
                        }
                    }
                    CompilerContext::PerfFunc => {
                        if let Some(index) = instrument.get_local_perf_variable(&variable_name) {
                            self.consume(TokenType::Equal, "Expected '='");
                            let variable_type = instrument.perf_local_type(index);
                            let expression_type = self.expression(instrument);
                            if variable_type != expression_type {
                                self.error_at_previous(format!("Expected {variable_type:?} to assign to local variable '{variable_name}' but got {expression_type:?}"));
                                return;
                            }
                            self.emit_constant(instrument, Value::int(index as i64));
                            self.emit_op(instrument, Op::AssignLocal);
                        }
                    }
                    _ => unreachable!(),
                }
            }
        } else {
            self.error_at_current("Expected statement".to_string());
        }

        self.consume(TokenType::Semicolon, "Expected ';'");
    }

    fn expression(&mut self, instrument: &mut Instrument) -> VariableType {
        self.term(instrument)
    }

    #[must_use]
    fn term(&mut self, instrument: &mut Instrument) -> VariableType {
        self.factor(instrument)
    }

    #[must_use]
    fn factor(&mut self, instrument: &mut Instrument) -> VariableType {
        self.call(instrument)
    }

    #[must_use]
    fn call(&mut self, instrument: &mut Instrument) -> VariableType {
        self.primary(instrument)
    }

    #[must_use]
    fn primary(&mut self, instrument: &mut Instrument) -> VariableType {
        if self.match_token(TokenType::Integer) {
            match self.previous.as_ref().unwrap().text().parse::<i64>() {
                Ok(value) => {
                    self.emit_constant(instrument, Value::int(value));
                    self.emit_op(instrument, Op::LoadConstant);
                    VariableType::Int
                }
                Err(err) => {
                    self.error_at_previous(format!("Error parsing Int: {err}"));
                    VariableType::None
                }
            }
        } else if self.match_token(TokenType::Float) {
            match self.previous.as_ref().unwrap().text().parse::<f32>() {
                Ok(value) => {
                    self.emit_constant(instrument, Value::float(value));
                    self.emit_op(instrument, Op::LoadConstant);
                    VariableType::Float
                }
                Err(err) => {
                    self.error_at_previous(format!("Error parsing Int: {err}"));
                    VariableType::None
                }
            }
        } else if self.match_token(TokenType::String) {
            match self.parse_string(self.previous.as_ref().unwrap().text()) {
                Ok(value) => {
                    self.emit_constant(instrument, Value::string(value));
                    self.emit_op(instrument, Op::LoadConstant);
                    VariableType::String
                }
                Err(err) => {
                    self.error_at_previous(format!("Error parsing Int: {err}"));
                    VariableType::None
                }
            }
        } else if self.match_token(TokenType::Identifier) {
            self.identifier(instrument)
        } else if self.match_token(TokenType::ParenOpen) {
            let expression_type = self.expression(instrument);
            self.consume(TokenType::ParenClose, "Expected ')'");
            expression_type
        } else {
            self.error_at_current("Invalid token at start of expression".to_string());
            VariableType::None
        }
    }

    fn is_escape_char(&self, c: char) -> Option<char> {
        match c {
            't' => Some('\t'),
            'r' => Some('\r'),
            'n' => Some('\n'),
            '\'' => Some('\''),
            '"' => Some('\"'),
            '\\' => Some('\\'),
            _ => None,
        }
    }

    fn parse_string(&self, string: &String) -> Result<String, ParseStringError> {
        let mut res = String::new();
        let text = string.as_bytes();
        for mut i in 1..text.len() - 1 {
            if text[i] == b'\\' {
                i += 1;
                if i == text.len() - 1 {
                    return Err(ParseStringError(
                        "Expected escape character but string terminated".to_string(),
                    ));
                }
                if let Some(escape_char) = self.is_escape_char(text[i] as char) {
                    res.push(escape_char);
                } else {
                    return Err(ParseStringError(format!(
                        "Unrecognised escape character '\\{}'",
                        text[i] as char
                    )));
                }
            } else {
                res.push(text[i] as char);
            }
        }

        Ok(res)
    }

    fn identifier(&mut self, instrument: &mut Instrument) -> VariableType {
        let var_name = self.previous.as_ref().unwrap().text();
        match self.context_stack.last().unwrap() {
            CompilerContext::InitFunc => {
                if let Some(index) = instrument.get_init_arg(var_name) {
                    self.emit_constant(instrument, Value::int(index as i64));
                    self.emit_op(instrument, Op::LoadArg);
                    instrument.init_arg_type(index)
                } else if let Some(index) = instrument.get_local_init_variable(var_name) {
                    self.emit_constant(instrument, Value::int(index as i64));
                    self.emit_op(instrument, Op::LoadLocal);
                    instrument.init_local_type(index)
                } else if let Some(index) = instrument.get_variable(var_name) {
                    self.emit_constant(instrument, Value::int(index as i64));
                    self.emit_op(instrument, Op::LoadMember);
                    instrument.member_type(index)
                } else {
                    self.error_at_previous(format!(
                        "No member variable, argument, or local variable found named '{var_name}'"
                    ));
                    VariableType::None
                }
            }
            CompilerContext::PerfFunc => {
                if let Some(index) = instrument.get_perf_arg(var_name) {
                    self.emit_constant(instrument, Value::int(index as i64));
                    self.emit_op(instrument, Op::LoadArg);
                    instrument.perf_arg_type(index)
                } else if let Some(index) = instrument.get_local_perf_variable(var_name) {
                    self.emit_constant(instrument, Value::int(index as i64));
                    self.emit_op(instrument, Op::LoadLocal);
                    instrument.perf_local_type(index)
                } else if let Some(index) = instrument.get_variable(var_name) {
                    self.emit_constant(instrument, Value::int(index as i64));
                    self.emit_op(instrument, Op::LoadMember);
                    instrument.member_type(index)
                } else {
                    self.error_at_previous(format!(
                        "No member variable, argument, or local variable found named '{var_name}'"
                    ));
                    VariableType::None
                }
            }
            _ => unreachable!(),
        }
    }

    fn score_block(&mut self) {
        self.context_stack.push(CompilerContext::ScoreBlock);
        self.consume(TokenType::BraceOpen, "Expected '{'");

        loop {
            if self.match_token(TokenType::BraceClose) {
                break;
            } else if self.match_token(TokenType::Identifier) {
                self.score_event();
            } else {
                self.error_at_current("Invalid token; expected instrument name or '}'".to_string())
            }

            if self.had_error {
                break;
            }
        }

        self.context_stack.pop();
    }

    fn score_event(&mut self) {
        let instrument_name = self.previous.as_ref().unwrap().text().clone();
        if !self.vm.has_instrument(&instrument_name) {
            self.error_at_previous(format!("No instrument named '{instrument_name}'"));
            return;
        }

        self.consume(TokenType::ParenOpen, "Expected '('");

        if !self.match_token(TokenType::Float) {
            self.error_at_current("Expected Float for start time".to_string());
            return;
        }

        let start_time = match self.previous.as_ref().unwrap().text().parse::<f32>() {
            Ok(value) => value,
            Err(err) => {
                self.error_at_previous(format!("Error parsing Float: {err}"));
                return;
            }
        };

        if !self.match_token(TokenType::Float) {
            self.error_at_current("Expected Float for duration".to_string());
            return;
        }

        let duration = match self.previous.as_ref().unwrap().text().parse::<f32>() {
            Ok(value) => value,
            Err(err) => {
                self.error_at_previous(format!("Error parsing Float: {err}"));
                return;
            }
        };

        let num_init_args = self.vm.instrument_num_init_args(&instrument_name);
        let num_perf_args = self.vm.instrument_num_perf_args(&instrument_name);
        let mut had_init_call = false;
        let mut had_perf_call = false;
        let mut init_args = Vec::<Value>::new();
        let mut perf_args = Vec::<Value>::new();

        loop {
            if self.match_token(TokenType::ParenClose) {
                break;
            } else if self.match_token(TokenType::InitIdent) {
                self.consume(TokenType::ParenOpen, "Expected '('");
                let mut arg_count = 0;
                loop {
                    if self.match_token(TokenType::ParenClose) {
                        break;
                    }

                    if arg_count == num_init_args {
                        self.error_at_current("Too many init args".to_string());
                        return;
                    }

                    match self.vm.instrument_init_arg_type(&instrument_name, arg_count) {
                        VariableType::Float => {
                            if !self.match_token(TokenType::Float) {
                                self.error_at_current(format!("Expected Float for init arg at position {arg_count}"));
                                return;
                            }

                            match self.previous.as_ref().unwrap().text().parse::<f32>() {
                                Ok(value) => init_args.push(Value::float(value)),
                                Err(err) => {
                                    self.error_at_previous(format!("Error parsing Float: {err}"));
                                    return;
                                }
                            }
                        }
                        VariableType::Int => {
                            if !self.match_token(TokenType::Integer) {
                                self.error_at_current(format!("Expected Int for init arg at position {arg_count}"));
                                return;
                            }

                            match self.previous.as_ref().unwrap().text().parse::<i64>() {
                                Ok(value) => init_args.push(Value::int(value)),
                                Err(err) => {
                                    self.error_at_previous(format!("Error parsing Int: {err}"));
                                    return;
                                }
                            }
                        }
                        VariableType::String => {
                            if !self.match_token(TokenType::String) {
                                self.error_at_current(format!("Expected String for init arg at position {arg_count}"));
                                return;
                            }

                            match self.parse_string(self.previous.as_ref().unwrap().text()) {
                                Ok(value) => init_args.push(Value::string(value)),
                                Err(err) => {
                                    self.error_at_previous(format!("Error parsing String: {err}"));
                                    return;
                                }
                            }
                        }
                        _ => unreachable!(),
                    }

                    arg_count += 1;
                }
                
                if arg_count != num_init_args {
                    self.error_at_previous(format!("Expected {num_init_args} init arguments but got {arg_count}"));
                    return;
                }

                had_init_call = true;
            } else if self.match_token(TokenType::PerfIdent) {
                self.consume(TokenType::ParenOpen, "Expected '('");
                let mut arg_count = 0;
                loop {
                    if self.match_token(TokenType::ParenClose) {
                        break;
                    }

                    if arg_count == num_perf_args {
                        self.error_at_current("Too many perf args".to_string());
                        return;
                    }

                    match self.vm.instrument_perf_arg_type(&instrument_name, arg_count) {
                        VariableType::Float => {
                            if !self.match_token(TokenType::Float) {
                                self.error_at_current(format!("Expected Float for perf arg at position {arg_count}"));
                                return;
                            }

                            match self.previous.as_ref().unwrap().text().parse::<f32>() {
                                Ok(value) => perf_args.push(Value::float(value)),
                                Err(err) => {
                                    self.error_at_previous(format!("Error parsing Float: {err}"));
                                    return;
                                }
                            }
                        }
                        VariableType::Int => {
                            if !self.match_token(TokenType::Integer) {
                                self.error_at_current(format!("Expected Int for perf arg at position {arg_count}"));
                                return;
                            }

                            match self.previous.as_ref().unwrap().text().parse::<i64>() {
                                Ok(value) => perf_args.push(Value::int(value)),
                                Err(err) => {
                                    self.error_at_previous(format!("Error parsing Int: {err}"));
                                    return;
                                }
                            }
                        }
                        VariableType::String => {
                            if !self.match_token(TokenType::String) {
                                self.error_at_current(format!("Expected String for perf arg at position {arg_count}"));
                                return;
                            }

                            match self.parse_string(self.previous.as_ref().unwrap().text()) {
                                Ok(value) => perf_args.push(Value::string(value)),
                                Err(err) => {
                                    self.error_at_previous(format!("Error parsing String: {err}"));
                                    return;
                                }
                            }
                        }
                        _ => unreachable!(),
                    }

                    arg_count += 1;
                }
                
                if arg_count != num_perf_args {
                    self.error_at_previous(format!("Expected {num_perf_args} perf arguments but got {arg_count}"));
                    return;
                }

                had_perf_call = true;
            } else {
                self.error_at_current("Invalid token; expected 'init' or 'perf'".to_string());
                return;
            }
        }

        if num_init_args > 0 && !had_init_call {
            self.error_at_previous(format!("init function for {instrument_name} takes {num_init_args} arguments but no init call was present in score event"));
            return;
        }

        if num_perf_args > 0 && !had_perf_call {
            self.error_at_previous(format!("perf function for {instrument_name} takes {num_perf_args} arguments but no perf call was present in score event"));
            return;
        }

        self.vm.add_score_event(&instrument_name, start_time, duration, init_args, perf_args);
        self.consume(TokenType::Semicolon, "Expected ';'");
    }

    fn had_error(&self) -> bool {
        self.had_error
    }

    fn error_at_current(&mut self, message: String) {
        let token = self.current.clone().unwrap();
        self.error(&token, message);
    }

    fn error_at_previous(&mut self, message: String) {
        let token = self.previous.clone().unwrap();
        self.error(&token, message);
    }

    fn error(&mut self, token: &Token, message: String) {
        self.had_error = true;

        if token.token_type() == TokenType::EndOfFile {
            eprintln!("{} at EOF: {}", "Compiler Error".red(), message);
        } else {
            eprintln!(
                "{} at '{}': {}",
                "Compiler Error".red(),
                token.text(),
                message
            );
        }

        eprintln!(
            "       --> {}:{}:{}",
            self.file_path,
            token.line(),
            token.column()
        );
        eprintln!("        |");
        eprintln!(
            "{:7} | {}",
            token.line(),
            self.scanner.get_code_at_line(token.line())
        );
        eprint!("        | ");

        for _ in 0..token.column() - 1 {
            eprint!(" ");
        }

        for _ in 0..token.len() {
            eprint!("{}", "^".red());
        }

        eprintln!();
    }
}
