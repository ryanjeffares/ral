use std::{error::Error, fmt, fs, path::Path};

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

    let mut real_time = false;
    let file_path = Path::new(&args[1]);
    for arg in args.iter().skip(2) {
        if arg == "--rt" {
            real_time = true;
        } else {
            usage();
            return Err(Box::new(ArgumentError(String::from("unknown argument"))));
        }
    }

    let code = fs::read_to_string(file_path)?;
    // let code = include_str!("../examples/test.ral").to_string();
    compiler::compiler::compile_and_run(code, String::from(file_path.to_str().unwrap()), real_time)
}

fn usage() {
    println!("Usage: ral <file_path>");
}
