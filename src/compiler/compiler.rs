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
    had_error: bool,
    context_stack: Vec<CompilerContext>,
    vm: VM,
}

pub fn compile_and_run(code: String, file_path: String) -> Result<(), Box<dyn Error>> {
    let mut compiler = Compiler {
        file_path,
        scanner: Scanner::new(code),
        had_error: false,
        context_stack: Vec::<CompilerContext>::new(),
        vm: VM::new(),
    };

    {
        let _timer = Timer::new("Compilation");
        compiler.compile();
    }

    if !compiler.had_error() {
        let _timer = Timer::new("Run");
        compiler.run()?;
    }

    Ok(())
}

impl Compiler {
    fn compile(&mut self) {
        self.context_stack.push(CompilerContext::TopLevel);
        while let Some(token) = self.advance() {
            match token.token_type() {
                TokenType::ErrorToken => self.error(&token, "Invalid token".to_string()),
                TokenType::InstrumentsIdent => self.instruments_block(),
                TokenType::ScoreIdent => self.score_block(),
                _ => self.error(&token, "Invalid token at top level".to_string()),
            }

            if self.had_error {
                eprintln!("Stopping parsing due to compilation errors");
                break;
            }
        }
    }

    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.vm.run()
    }

    fn had_error(&self) -> bool {
        self.had_error
    }

    fn emit_op(&mut self, instrument: &mut Instrument, op: Op) {
        match self.context_stack.last().unwrap() {
            CompilerContext::InitFunc => instrument.emit_init_op(op),
            CompilerContext::PerfFunc => instrument.emit_perf_op(op),
            _ => unreachable!(),
        }
    }

    fn emit_constant(&mut self, instrument: &mut Instrument, value: Value) {
        match self.context_stack.last().unwrap() {
            CompilerContext::InitFunc => instrument.emit_init_constant(value),
            CompilerContext::PerfFunc => instrument.emit_perf_constant(value),
            _ => unreachable!(),
        }
    }

    fn advance(&mut self) -> Option<Token> {
        let next = self.scanner.scan_token();
        match next {
            Some(token) => {
                if token.token_type() == TokenType::ErrorToken {
                    self.error(&token, "Invalid token".to_string());
                    None
                } else {
                    Some(token)
                }
            }
            None => None,
        }
    }

    fn consume(&mut self, expected: TokenType, message: &'static str) {
        if let Some(token) = self.advance() {
            if token.token_type() != expected {
                self.error(&token, message.to_string());
            }
        } else {
            self.error_missing_token();
        }
    }

    fn instruments_block(&mut self) {
        self.context_stack.push(CompilerContext::InstrumentsBlock);

        if let Some(next) = self.advance() {
            if next.token_type() != TokenType::BraceOpen {
                self.error(&next, "Expected '{' after 'instruments'".to_string());
                return;
            }

            loop {
                if let Some(next) = self.advance() {
                    if next.token_type() == TokenType::BraceClose {
                        break;
                    }

                    if next.token_type() != TokenType::Identifier {
                        self.error(&next, "Expected instrument name".to_string());
                        break;
                    }

                    self.instrument(next.text().clone());
                } else {
                    self.error_missing_token();
                    break;
                }
            }
        } else {
            self.error_missing_token();
        }

        self.context_stack.pop();
    }

    fn score_block(&mut self) {
        self.context_stack.push(CompilerContext::ScoreBlock);

        if let Some(next) = self.advance() {
            if next.token_type() != TokenType::BraceOpen {
                self.error(&next, "Expected '{' after 'score'".to_string());
                return;
            }

            loop {
                if let Some(next) = self.advance() {
                    if next.token_type() == TokenType::BraceClose {
                        break;
                    }

                    if next.token_type() != TokenType::Identifier {
                        self.error(&next, "Expected instrument name".to_string());
                        break;
                    }

                    self.score_event(&next);
                } else {
                    self.error_missing_token();
                    break;
                }
            }
        } else {
            self.error_missing_token();
        }
    }

    fn score_event(&mut self, token: &Token) {
        let instrument_name = token.text();
        if self.vm.has_instrument(instrument_name) {
            self.consume(TokenType::ParenOpen, "Expected '('");
            let start_time;
            let duration;

            // start_time duration init(...) perf(...)
            if let Some(next) = self.advance() {
                if next.token_type() != TokenType::Float {
                    self.error(&next, "Expected Float for start time".to_string());
                    return;
                }

                let res = next.text().parse::<f32>();
                if res.is_err() {
                    self.error(&next, "Unable to parse Float".to_string());
                }
                start_time = res.unwrap();
            } else {
                self.error_missing_token();
                return;
            }

            if let Some(next) = self.advance() {
                if next.token_type() != TokenType::Float {
                    self.error(&next, "Expected Float for duration".to_string());
                    return;
                }

                let res = next.text().parse::<f32>();
                if res.is_err() {
                    self.error(&next, "Unable to parse Float".to_string());
                }
                duration = res.unwrap();
            } else {
                self.error_missing_token();
                return;
            }

            // instruments that have a init/perf function that takes any arguments
            // require init(...)/perf(...) calls in their score event.
            // if an instruments init/perf function takes no args, you may omit it completely from the score event,
            // or put an empty init()/perf()

            let num_init_args = self.vm.instrument_num_init_args(instrument_name);
            let num_perf_args = self.vm.instrument_num_perf_args(instrument_name);
            let mut had_init_call = false;
            let mut had_perf_call = false;
            let mut init_args = Vec::<Value>::new();
            let mut perf_args = Vec::<Value>::new();

            while let Some(next) = self.advance() {
                match next.token_type() {
                    TokenType::ParenClose => {
                        if !had_init_call && num_init_args > 0 {
                            self.error(&next, format!("Instrument '{instrument_name}' expects {num_init_args} init args but no init call was given in score event"));
                        }
                        if !had_perf_call && num_perf_args > 0 {
                            self.error(&next, format!("Instrument '{instrument_name}' expects {num_perf_args} perf args but no perf call was given in score event"));
                        }

                        if let Some(next) = self.advance() {
                            if next.token_type() != TokenType::Semicolon {
                                self.error(&next, "Expected ';'".to_string());
                                return;
                            }
                            break;
                        } else {
                            self.error_missing_token();
                            return;
                        }
                    }
                    TokenType::InitIdent => {
                        if let Some(next) = self.advance() {
                            if next.token_type() != TokenType::ParenOpen {
                                self.error(&next, "Expected '('".to_string());
                                return;
                            }

                            let mut arg_count = 0;
                            while let Some(next) = self.advance() {
                                match next.token_type() {
                                    TokenType::ParenClose => {
                                        if arg_count != num_init_args {
                                            self.error(&next, format!("Instrument '{instrument_name}' expects {num_init_args} init args but got {arg_count} in score event"));
                                            return;
                                        }
                                        break;
                                    }
                                    TokenType::Integer | TokenType::Float | TokenType::String => {
                                        if arg_count >= num_init_args {
                                            self.error(&next, format!("Too many init args for instrument '{instrument_name}', expected {num_init_args}"));
                                            return;
                                        }

                                        let expected_type = self
                                            .vm
                                            .instrument_init_arg_type(instrument_name, arg_count);
                                        if expected_type
                                            != match next.token_type() {
                                                TokenType::Integer => VariableType::Int,
                                                TokenType::Float => VariableType::Float,
                                                TokenType::String => VariableType::String,
                                                _ => unreachable!(),
                                            }
                                        {
                                            self.error(&next, format!("Instrument '{instrument_name}' expected {expected_type:?} for init arg at position {arg_count} but got {:?}", next.token_type()));
                                            return;
                                        }

                                        match next.token_type() {
                                            TokenType::Integer => {
                                                if let Some(value) = self.parse_int(&next) {
                                                    init_args.push(Value::int(value))
                                                } else {
                                                    return;
                                                }
                                            }
                                            TokenType::Float => {
                                                if let Some(value) = self.parse_float(&next) {
                                                    init_args.push(Value::float(value))
                                                } else {
                                                    return;
                                                }
                                            }
                                            TokenType::String => {
                                                if let Some(value) = self.parse_string(&next) {
                                                    init_args.push(Value::string(value))
                                                } else {
                                                    return;
                                                }
                                            }
                                            _ => unreachable!(),
                                        }

                                        arg_count += 1;
                                    }
                                    _ => self.error(
                                        &next,
                                        "Invalid type for init arg in score event".to_string(),
                                    ),
                                }
                            }
                        } else {
                            self.error_missing_token();
                        }

                        had_init_call = true;
                    }
                    TokenType::PerfIdent => {
                        if let Some(next) = self.advance() {
                            if next.token_type() != TokenType::ParenOpen {
                                self.error(&next, "Expected '('".to_string());
                                return;
                            }

                            let mut arg_count = 0;
                            while let Some(next) = self.advance() {
                                match next.token_type() {
                                    TokenType::ParenClose => {
                                        if arg_count != num_perf_args {
                                            self.error(&next, format!("Instrument '{instrument_name}' expects {num_perf_args} perf args but got {arg_count} in score event"));
                                            return;
                                        }
                                        break;
                                    }
                                    TokenType::Integer | TokenType::Float | TokenType::String => {
                                        if arg_count >= num_perf_args {
                                            self.error(&next, format!("Too many perf args for instrument '{instrument_name}', expected {num_perf_args}"));
                                            return;
                                        }

                                        let expected_type = self
                                            .vm
                                            .instrument_perf_arg_type(instrument_name, arg_count);
                                        if expected_type
                                            != match next.token_type() {
                                                TokenType::Integer => VariableType::Int,
                                                TokenType::Float => VariableType::Float,
                                                TokenType::String => VariableType::String,
                                                _ => unreachable!(),
                                            }
                                        {
                                            self.error(&next, format!("Instrument '{instrument_name}' expected {expected_type:?} for perf arg at position {arg_count} but got {:?}", next.token_type()));
                                            return;
                                        }

                                        match next.token_type() {
                                            TokenType::Integer => {
                                                if let Some(value) = self.parse_int(&next) {
                                                    perf_args.push(Value::int(value))
                                                } else {
                                                    return;
                                                }
                                            }
                                            TokenType::Float => {
                                                if let Some(value) = self.parse_float(&next) {
                                                    perf_args.push(Value::float(value))
                                                } else {
                                                    return;
                                                }
                                            }
                                            TokenType::String => {
                                                if let Some(value) = self.parse_string(&next) {
                                                    perf_args.push(Value::string(value))
                                                } else {
                                                    return;
                                                }
                                            }
                                            _ => unreachable!(),
                                        }

                                        arg_count += 1;
                                    }
                                    _ => self.error(
                                        &next,
                                        "Invalid type for perf arg in score event".to_string(),
                                    ),
                                }
                            }
                        } else {
                            self.error_missing_token();
                        }

                        had_perf_call = true;
                    }
                    _ => self.error(&next, "Invalid token in score event".to_string()),
                }
            }

            println!("start: {start_time}, duration: {duration}");
            println!("init args: {init_args:?}");
            println!("perf args: {perf_args:?}");

            self.vm
                .add_score_event(instrument_name, start_time, duration, init_args, perf_args);
        } else {
            self.error(token, format!("No instrument named '{instrument_name}'"));
        }
    }

    fn instrument(&mut self, instrument_name: String) {
        self.context_stack.push(CompilerContext::Instrument);

        if let Some(next) = self.advance() {
            if next.token_type() != TokenType::BraceOpen {
                self.error(&next, "Expected '{' after 'instruments'".to_string());
                return;
            }

            let mut instrument = Instrument::new(instrument_name);
            while let Some(next) = self.advance() {
                if next.token_type() == TokenType::BraceClose {
                    break;
                }

                match next.token_type() {
                    TokenType::Identifier => {
                        let variable_name = next.text();
                        if let Some(next) = self.advance() {
                            if next.token_type() != TokenType::Colon {
                                self.error(&next, "Expected ':' after variable name".to_string());
                                return;
                            }

                            if let Some(next) = self.advance() {
                                if !next.token_type().is_type_ident() {
                                    self.error(&next, "Expected type identifier".to_string());
                                    return;
                                }

                                instrument.add_variable(
                                    variable_name.to_owned(),
                                    match next.token_type() {
                                        TokenType::IntIdent => VariableType::Int,
                                        TokenType::FloatIdent => VariableType::Float,
                                        TokenType::AudioIdent => VariableType::Audio,
                                        _ => unreachable!(),
                                    },
                                );

                                if let Some(next) = self.advance() {
                                    if next.token_type() != TokenType::Semicolon {
                                        self.error(&next, "Expected ';'".to_string());
                                    }
                                } else {
                                    self.error_missing_token();
                                }
                            } else {
                                self.error_missing_token();
                            }
                        }
                    }
                    TokenType::InitIdent => self.init_func(&mut instrument),
                    TokenType::PerfIdent => self.perf_func(&mut instrument),
                    _ => self.error(
                        &next,
                        "Expected identifier, 'init' or 'ident' at instrument top level"
                            .to_string(),
                    ),
                }
            }

            self.vm.add_instrument(instrument);
        } else {
            self.error_missing_token();
        }

        self.context_stack.pop();
    }

    fn init_func(&mut self, instrument: &mut Instrument) {
        self.context_stack.push(CompilerContext::InitFunc);

        if let Some(next) = self.advance() {
            if next.token_type() == TokenType::ParenOpen {
                let mut arg_count = 0;
                while let Some(next) = self.advance() {
                    if next.token_type() == TokenType::ParenClose {
                        break;
                    }

                    if next.token_type() != TokenType::Identifier {
                        self.error(&next, "Expected identifier".to_string());
                        break;
                    }

                    let arg_name = next.text();
                    self.consume(TokenType::Colon, "Expected ':' after argument name");

                    if let Some(next) = self.advance() {
                        if !next.token_type().is_type_ident() {
                            self.error(&next, "Expected type identifier".to_string());
                            break;
                        }
                        instrument.add_init_arg(
                            arg_count,
                            arg_name.to_owned(),
                            match next.token_type() {
                                TokenType::IntIdent => VariableType::Int,
                                TokenType::FloatIdent => VariableType::Float,
                                TokenType::StringIdent => VariableType::String,
                                _ => {
                                    self.error(&next, "Invalid type for init arg".to_string());
                                    break;
                                }
                            },
                        );

                        arg_count += 1;

                        if let Some(next) = self.advance() {
                            match next.token_type() {
                                TokenType::ParenClose => break,
                                TokenType::Comma => (),
                                _ => self.error(&next, "Expected ',' or ')'".to_string()),
                            }
                        } else {
                            self.error_missing_token();
                            return;
                        }
                    } else {
                        self.error_missing_token();
                        return;
                    }
                }
            } else {
                self.error(&next, "Expected '('".to_string());
                return;
            }

            self.consume(TokenType::BraceOpen, "Expected '{");

            while let Some(next) = self.advance() {
                match next.token_type() {
                    TokenType::BraceClose => break,
                    _ => self.declaration(&next, instrument),
                }
            }
        } else {
            self.error_missing_token();
        }

        self.context_stack.pop();
    }

    fn perf_func(&mut self, instrument: &mut Instrument) {
        self.context_stack.push(CompilerContext::PerfFunc);

        if let Some(next) = self.advance() {
            if next.token_type() == TokenType::ParenOpen {
                let mut arg_count = 0;
                while let Some(next) = self.advance() {
                    if next.token_type() == TokenType::ParenClose {
                        break;
                    }

                    if next.token_type() != TokenType::Identifier {
                        self.error(&next, "Expected identifier".to_string());
                        break;
                    }

                    let arg_name = next.text();
                    self.consume(TokenType::Colon, "Expected ':' after argument name");

                    if let Some(next) = self.advance() {
                        if !next.token_type().is_type_ident() {
                            self.error(&next, "Expected type identifier".to_string());
                            break;
                        }
                        instrument.add_perf_arg(
                            arg_count,
                            arg_name.to_owned(),
                            match next.token_type() {
                                TokenType::IntIdent => VariableType::Int,
                                TokenType::FloatIdent => VariableType::Float,
                                TokenType::StringIdent => VariableType::String,
                                _ => {
                                    self.error(&next, "Invalid type for perf arg".to_string());
                                    break;
                                }
                            },
                        );

                        arg_count += 1;

                        if let Some(next) = self.advance() {
                            match next.token_type() {
                                TokenType::ParenClose => break,
                                TokenType::Comma => (),
                                _ => self.error(&next, "Expected ',' or ')'".to_string()),
                            }
                        } else {
                            self.error_missing_token();
                            return;
                        }
                    } else {
                        self.error_missing_token();
                        return;
                    }
                }
            }

            self.consume(TokenType::BraceOpen, "Expected '{'");
            while let Some(next) = self.advance() {
                match next.token_type() {
                    TokenType::BraceClose => break,
                    _ => self.declaration(&next, instrument),
                }
            }
        } else {
            self.error_missing_token();
        }

        self.context_stack.pop();
    }

    fn declaration(&mut self, token: &Token, instrument: &mut Instrument) {
        match token.token_type() {
            TokenType::Local => self.local_declaration(token, instrument),
            _ => self.statement(token, instrument),
        }
    }

    fn local_declaration(&mut self, token: &Token, instrument: &mut Instrument) {}

    fn statement(&mut self, token: &Token, instrument: &mut Instrument) {
        match token.token_type() {
            TokenType::Print | TokenType::PrintLn => self.print_statement(token, instrument),
            _ => self.expression_statement(token, instrument),
        }
    }

    fn print_statement(&mut self, token: &Token, instrument: &mut Instrument) {
        let print_line = token.token_type() == TokenType::PrintLn;

        if let Some(next) = self.advance() {
            if next.token_type() != TokenType::ParenOpen {
                self.error(&next, "Expected '('".to_string());
                return;
            }

            if let Some(next) = self.advance() {
                let context = *self.context_stack.last().unwrap();
                if next.token_type() == TokenType::ParenClose {
                    self.emit_op(
                        instrument,
                        if print_line {
                            Op::PrintLnEmpty
                        } else {
                            Op::PrintEmpty
                        },
                    );
                } else if let Some(next) = self.expression(false, &next, instrument) {
                    if next.token_type() != TokenType::ParenClose {
                        self.error(&next, "Expected ')'".to_string());
                        return;
                    }

                    self.emit_op(instrument, if print_line { Op::PrintLn } else { Op::Print });
                }

                self.consume(TokenType::Semicolon, "Expected ';'");
            } else {
                self.error_missing_token();
            }
        } else {
            self.error_missing_token();
        }
    }

    fn expression_statement(&mut self, token: &Token, instrument: &mut Instrument) {
        if token.token_type().is_literal() || token.token_type().is_operator() {
            self.error(
                token,
                "Expected identifier or keyword at start of expression".to_string(),
            );
            self.advance(); // consume illegal token
        } else if let Some(next) = self.expression(true, token, instrument) {
            if next.token_type() != TokenType::Semicolon {
                self.error(&next, "Expected ';' after expression statement".to_string());
            }
        }
    }

    fn expression(
        &mut self,
        can_assign: bool,
        token: &Token,
        instrument: &mut Instrument,
    ) -> Option<Token> {
        if token.token_type() == TokenType::Identifier {
            if let Some(current) = self.primary(token, instrument) {
                if current.token_type() == TokenType::Equal {
                    if !can_assign {
                        self.error(
                            &current,
                            "Assignment is not valid in this context".to_string(),
                        );
                        return None;
                    }

                    if let Some(next) = self.advance() {
                        let next = self.expression(false, &next, instrument);

                        let context = *self.context_stack.last().unwrap();
                        if context == CompilerContext::InitFunc {
                            if let Some(index) = instrument.has_variable(token.text()) {
                                instrument.emit_init_constant(Value::int(index as i64));
                                instrument.emit_init_op(Op::AssignMember);
                            } else if instrument.has_init_arg(token.text()).is_some() {
                                self.error(token, "Cannot reassign to init arg".to_string());
                                return None;
                            } else {
                                self.error(
                                    token,
                                    format!(
                                        "Cannot find variable '{}' in this scope",
                                        token.text()
                                    ),
                                );
                                return None;
                            }
                        } else if context == CompilerContext::PerfFunc {
                            if let Some(index) = instrument.has_variable(token.text()) {
                                instrument.emit_perf_constant(Value::int(index as i64));
                                instrument.emit_perf_op(Op::AssignMember);
                            } else if instrument.has_perf_arg(token.text()).is_some() {
                                self.error(token, "Cannot reassign to perf arg".to_string());
                                return None;
                            } else {
                                self.error(
                                    token,
                                    format!(
                                        "Cannot find variable '{}' in this scope",
                                        token.text()
                                    ),
                                );
                                return None;
                            }
                        } else {
                            unreachable!();
                        }

                        return next;
                    } else {
                        self.error_missing_token();
                        return None;
                    }
                }

                Some(current)
            } else {
                self.error_missing_token();
                None
            }
        } else {
            // TODO: full recursive descent
            self.primary(token, instrument)
        }
    }

    fn primary(&mut self, token: &Token, instrument: &mut Instrument) -> Option<Token> {
        match token.token_type() {
            TokenType::Identifier => self.identifier(token, instrument),
            TokenType::String => self.string(token, instrument),
            _ => todo!(),
        }
    }

    fn identifier(&mut self, token: &Token, instrument: &mut Instrument) -> Option<Token> {
        if let Some(next) = self.advance() {
            if next.token_type() != TokenType::Equal {
                let context = *self.context_stack.last().unwrap();
                if context == CompilerContext::InitFunc {
                    if let Some(index) = instrument.has_variable(token.text()) {
                        instrument.emit_init_constant(Value::int(index as i64));
                        instrument.emit_init_op(Op::LoadMember);
                    } else if let Some(index) = instrument.has_init_arg(token.text()) {
                        instrument.emit_init_constant(Value::int(index as i64));
                        instrument.emit_init_op(Op::LoadArg);
                    } else {
                        self.error(
                            token,
                            format!("Couldn't find variable '{}' in this scope", token.text()),
                        );
                    }
                } else if context == CompilerContext::PerfFunc {
                    if let Some(index) = instrument.has_variable(token.text()) {
                        instrument.emit_perf_constant(Value::int(index as i64));
                        instrument.emit_perf_op(Op::LoadMember);
                    } else if let Some(index) = instrument.has_perf_arg(token.text()) {
                        instrument.emit_perf_constant(Value::int(index as i64));
                        instrument.emit_perf_op(Op::LoadArg);
                    } else {
                        self.error(
                            token,
                            format!("Couldn't find variable '{}' in this scope", token.text()),
                        );
                    }
                } else {
                    unreachable!();
                }
            }

            Some(next)
        } else {
            self.error_missing_token();
            None
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

    fn string(&mut self, token: &Token, instrument: &mut Instrument) -> Option<Token> {
        if let Some(res) = self.parse_string(token) {            
            self.emit_constant(instrument, Value::string(res));
            self.emit_op(instrument, Op::LoadConstant);
            self.advance()
        } else {
            None
        }
    }

    fn parse_string(&mut self, token: &Token) -> Option<String> {
        let mut res = String::new();
        let text = token.text().as_bytes();
        for mut i in 1..text.len() - 1 {
            if text[i] == b'\\' {
                i += 1;
                if i == text.len() - 1 {
                    self.error(
                        token,
                        "Expected escape character but string terminated".to_string(),
                    );
                    return None;
                }
                if let Some(escape_char) = self.is_escape_char(text[i] as char) {
                    res.push(escape_char);
                } else {
                    self.error(
                        token,
                        format!("Unrecognised escape character '\\{}'", text[i] as char),
                    );
                    return None;
                }
            } else {
                res.push(text[i] as char);
            }
        }

        Some(res)
    }

    fn parse_int(&mut self, token: &Token) -> Option<i64> {
        let res = token.text().parse::<i64>();
        match res {
            Err(err) => {
                self.error(token, format!("Failed to parse Int: {err}"));
                None
            }
            Ok(value) => Some(value),
        }
    }

    fn parse_float(&mut self, token: &Token) -> Option<f32> {
        let res = token.text().parse::<f32>();
        match res {
            Err(err) => {
                self.error(token, format!("Failed to parse Float: {err}"));
                None
            }
            Ok(value) => Some(value),
        }
    }

    fn error(&mut self, token: &Token, message: String) {
        self.had_error = true;

        eprintln!(
            "{} at '{}': {}",
            "Compiler Error".red(),
            token.text(),
            message
        );
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

    fn error_missing_token(&mut self) {
        self.had_error = true;
        eprintln!("{}: Expected token but got EOF", "Compiler Error".red());
        eprintln!("       --> {}:{}", self.file_path, self.scanner.line(),);
        eprintln!("        |");
        eprintln!(
            "{:7} | {}",
            self.scanner.line(),
            self.scanner.get_code_at_line(self.scanner.line())
        );
        eprintln!("        | ");
    }
}
