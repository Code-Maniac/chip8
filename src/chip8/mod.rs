mod interpreter;

use clap::Parser;
use std::path::Path;
use std::time::Instant;

use crate::chip8::interpreter::Interpreter;

/// Chip8 Interpreter
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    romfile: String,
}

pub fn start() {
    println!("Hello World!");

    // parse the arguments
    let args = Args::parse();

    // check if the romfile exists and if it does then load it
    let path = Path::new(&args.romfile);
    if !path.exists() {
        println!("Romfile does not exist");
        std::process::exit(-1);
    }

    // the start time
    let start_time = Instant::now();

    // setup the chip8 interpretter
    let mut interp = Interpreter::load(path).unwrap();

    loop {
        interp.update(&start_time);
    }

    // setup the window with sdl2

    // run

    // std::process::exit(0);
}
