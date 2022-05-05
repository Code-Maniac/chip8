# chip8

A chip8 interpretter written in rust

## Installation
Clone the repository and build with cargo

## Usage
```
USAGE:
    chip8 [OPTIONS] <ROMFILE>

ARGS:
    <ROMFILE>    

OPTIONS:
    -c, --clockspeed <CLOCKSPEED>    The clock speed on the "cpu" in MHz, this is the number of
                                     chip8 opcodes that will be processed per second [default: 400]
    -h, --help                       Print help information
    -p, --pixelsize <PIXELSIZE>      The number of pixels that each "chip8" pixel is represented by
                                     on the window canvas [default: 8]
    -V, --version                    Print version information
```

# License
[MIT](https://choosealicense.com/licenses/mit/)
