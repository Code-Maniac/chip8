extern crate rand;
extern crate sdl2;

mod audio;
mod colors;
mod interpreter;
mod keyboard;
mod video;

use clap::Parser;
use std::path::Path;
use std::time::Instant;

use interpreter::Interpreter;

/// Chip8 Interpreter
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    romfile: String,

    /// The number of pixels that each "chip8" pixel is represented by on the
    /// window canvas
    #[clap(short, long, default_value_t = 8)]
    pixelsize: usize,

    /// The clock speed on the "cpu" in MHz, this is the number of chip8 opcodes
    /// that will be processed per second
    #[clap(short, long, default_value = "400")]
    clockspeed: u32,
}

pub fn start() {
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
    let sdl_context = sdl2::init().unwrap();
    let mut interp = Interpreter::load(
        &sdl_context,
        path,
        args.pixelsize,
        args.clockspeed,
        &start_time,
    )
    .unwrap();

    loop {
        interp.update(&start_time);
    }
}
