use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::keyboard::Scancode;
use sdl2::Sdl;

// each scancode needs to be at a specific index
const SCAN_CODES: &'static [Scancode; 0x10] = &[
    Scancode::X,
    Scancode::Num1,
    Scancode::Num2,
    Scancode::Num3,
    Scancode::Q,
    Scancode::W,
    Scancode::E,
    Scancode::A,
    Scancode::S,
    Scancode::D,
    Scancode::Z,
    Scancode::C,
    Scancode::Num4,
    Scancode::R,
    Scancode::F,
    Scancode::V,
];

pub struct KeyboardDevice<'a> {
    sdl_context: &'a Sdl,

    // registers for the keys
    keys: [bool; 0x10],
}

impl<'a> KeyboardDevice<'a> {
    pub fn new(sdl_context: &'a Sdl) -> Self {
        KeyboardDevice {
            sdl_context,
            keys: [false; 0x10],
        }
    }

    pub fn read_keys(&mut self) {
        for i in 0x0..0x10 {
            let code = SCAN_CODES[i];

            if self
                .sdl_context
                .event_pump()
                .unwrap()
                .keyboard_state()
                .is_scancode_pressed(code)
            {
                self.keys[i] = true;
            }
        }
    }

    pub fn clear_keys(&mut self) {
        for i in 0x0..0x10 {
            self.keys[i] = false;
        }
    }

    pub fn is_key_pressed(&self, keycode: u8) -> bool {
        self.keys[keycode as usize]
    }

    pub fn get_key_event(&self) -> u8 {
        // now keep polling events until a key down happens on:
        // REAL -> CHIP8
        // 1234 -> 123B
        // qwer -> 456C
        // asdf -> 789D
        // zxcv -> A0BF
        let mut key: u8 = 0xFF;
        while key == 0xFF {
            for event in self.sdl_context.event_pump().unwrap().poll_iter() {
                match event {
                    // row one
                    Event::KeyDown {
                        keycode: Some(Keycode::Num1),
                        repeat: false,
                        ..
                    } => {
                        key = 0x1;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Num2),
                        repeat: false,
                        ..
                    } => {
                        key = 0x2;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Num3),
                        repeat: false,
                        ..
                    } => {
                        key = 0x3;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Num4),
                        repeat: false,
                        ..
                    } => {
                        key = 0xC;
                    }
                    // row two
                    Event::KeyDown {
                        keycode: Some(Keycode::Q),
                        repeat: false,
                        ..
                    } => {
                        key = 0x4;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::W),
                        repeat: false,
                        ..
                    } => {
                        key = 0x5;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::E),
                        repeat: false,
                        ..
                    } => {
                        key = 0x6;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::R),
                        repeat: false,
                        ..
                    } => {
                        key = 0xD;
                    }
                    // row three
                    Event::KeyDown {
                        keycode: Some(Keycode::A),
                        repeat: false,
                        ..
                    } => {
                        key = 0x7;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::S),
                        repeat: false,
                        ..
                    } => {
                        key = 0x8;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::D),
                        repeat: false,
                        ..
                    } => {
                        key = 0x9;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::F),
                        repeat: false,
                        ..
                    } => {
                        key = 0xE;
                    }
                    // row four
                    Event::KeyDown {
                        keycode: Some(Keycode::Z),
                        repeat: false,
                        ..
                    } => {
                        key = 0xA;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::X),
                        repeat: false,
                        ..
                    } => {
                        key = 0x0;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::C),
                        repeat: false,
                        ..
                    } => {
                        key = 0xB;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::V),
                        repeat: false,
                        ..
                    } => {
                        key = 0xF;
                    }
                    _ => (), // ignore other events
                }
            }
        }
        key
    }
}
