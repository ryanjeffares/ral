#![feature(once_cell)]

use std::{error::Error, fmt, path::Path, fs};

use runtime::vm::OutputTarget;

mod audio;
mod compiler;
mod runtime;
mod utils;

#[derive(Debug)]
struct ArgumentError(String);

impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid command line arguments: {}", self.0)
    }
}

impl Error for ArgumentError {}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
        return Err(Box::new(ArgumentError(String::from(
            "insufficient arguments",
        ))));
    }

    let mut output_target = OutputTarget::None;
    let file_path = Path::new(&args[1]);
    for arg in args.iter().skip(2) {
        if arg == "--dac" {
            if output_target != OutputTarget::None {
                usage();
                return Err(Box::new(ArgumentError(String::from(
                    "output target is mutually exclusive",
                ))));
            }
            output_target = OutputTarget::Dac;
        } else if arg == "--file" {
            if output_target != OutputTarget::None {
                usage();
                return Err(Box::new(ArgumentError(String::from(
                    "output target is mutually exclusive",
                ))));
            }
            output_target = OutputTarget::File;
        } else {
            usage();
            return Err(Box::new(ArgumentError(String::from("unknown argument"))));
        }
    }

    let code = fs::read_to_string(file_path)?;
    // let code = include_str!("../examples/wav_player.ral").to_string();
    compiler::compiler::compile_and_run(
        code,
        String::from(file_path.to_str().unwrap()),
        output_target,
    )
}

fn usage() {
    println!("Usage: ral <file_path>");
}
