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

            self.keys[i] = self
                .sdl_context
                .event_pump()
                .unwrap()
                .keyboard_state()
                .is_scancode_pressed(code);
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

    pub fn get_key_press(&self) -> Option<u8> {
        for i in 0x0..0x10 {
            if self.keys[i] {
                return Some(i as u8);
            }
        }
        None
    }
}
