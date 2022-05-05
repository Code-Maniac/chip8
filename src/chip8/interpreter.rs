use rand::Rng;
use sdl2::event::Event;
use sdl2::Sdl;
use std::fs;
use std::num::Wrapping;
use std::path::Path;
use std::time::Instant;

use super::audio::AudioDevice;
use super::keyboard::KeyboardDevice;
use super::video::VideoDevice;

// define constants for using the memory
// Chip 8 has 4096 bytes
const MEM_SIZE: usize = 0x1000;

// 16 registers
const REGISTERS_SIZE: usize = 0x10;

const PROGRAM_START: usize = 0x200;

const STACK_SLOTS: usize = 64;

// fonts will be loaded into memory location 0
const FONT_START: usize = 0x0;
// each font character is 5 bytes in size
const FONT_CHAR_SIZE: usize = 5;
// there are character 0,1,2,3,4,5,6,7,8,9,A,B,C,D,E,F available
const FONT_CHAR_COUNT: usize = 0x10;
// the static font data that will be loaded into the memory
const FONT_DATA: &'static [u8; FONT_CHAR_SIZE * FONT_CHAR_COUNT] = &[
    0xF0, 0x90, 0x90, 0x90, 0xF0, // "0"
    0x20, 0x60, 0x20, 0x20, 0x70, // "1"
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // "2"
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // "3"
    0x90, 0x90, 0xF0, 0x10, 0x10, // "4"
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // "5"
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // "6"
    0xF0, 0x10, 0x20, 0x40, 0x40, // "7"
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // "8"
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // "9"
    0xF0, 0x90, 0xF0, 0x90, 0x90, // "A"
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // "B"
    0xF0, 0x80, 0x80, 0x80, 0xF0, // "C"
    0xE0, 0x90, 0x90, 0x90, 0xE0, // "D"
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // "E"
    0xF0, 0x80, 0xF0, 0x80, 0x80, // "F"
];

// the number of ticks between updates
const UPDATE_TICKS: u128 = 16667;

pub struct Interpreter<'a> {
    // the sdl context
    sdl_context: &'a Sdl,

    // the video device used for drawing to screen
    video_device: VideoDevice,

    // the audio device used for the beeps
    audio_device: AudioDevice,

    // the keyboard device used to handle key input
    keyboard_device: KeyboardDevice<'a>,

    // the number of ticks between opcodes
    opcode_ticks: u128,

    // the memory
    memory: [u8; MEM_SIZE],

    // 16 variables variables
    registers: [Wrapping<u8>; REGISTERS_SIZE],

    // stack for function entry/return
    stack: Vec<usize>,

    // the stack pointer
    sp: usize,

    // Program counter - represents the current position in execution of the
    // program
    pc: usize,

    // 16 bit register for memory addresses
    i: usize,

    // timers
    delay_timer: u8,
    sound_timer: u8,

    // Time of the next opcode
    next_opcode_time: Wrapping<u128>,

    // the update time controls when the render and the timer decrement happens
    // this happens at a rate of 60hz
    next_update_time: Wrapping<u128>,
}

impl<'a> Interpreter<'a> {
    pub fn load(
        sdl_context: &'a Sdl,
        romfile: &Path,
        pixelsize: usize,
        clockspeed: u32,
        start_time: &Instant,
    ) -> Result<Interpreter<'a>, &'static str> {
        let video_device = VideoDevice::new(&sdl_context, pixelsize);
        let audio_device = AudioDevice::new(&sdl_context);
        let keyboard_device = KeyboardDevice::new(&sdl_context);

        let mut interp = Interpreter {
            sdl_context,
            video_device,
            audio_device,
            keyboard_device,
            opcode_ticks: (1000000.0 / (clockspeed as f64)) as u128,
            memory: [0; MEM_SIZE],
            registers: [Wrapping(0); REGISTERS_SIZE],
            stack: Vec::new(),
            sp: 0,
            pc: PROGRAM_START,
            i: 0,
            delay_timer: 0,
            sound_timer: 0,
            next_opcode_time: Wrapping(start_time.elapsed().as_micros()),
            next_update_time: Wrapping(start_time.elapsed().as_micros()),
        };

        // load the romfile into the program data in the interpretter memory
        let data = fs::read(romfile).expect("Could not load romfile");
        for (i, v) in data.iter().enumerate() {
            interp.memory[PROGRAM_START + i] = *v;
        }

        // load the fonts into interpretter area of memory
        for i in 0..FONT_DATA.len() {
            interp.memory[i] = FONT_DATA[i];
        }

        Ok(interp)
    }

    // function to do next cpu cycle
    pub fn update(&mut self, start_time: &Instant) {
        let elapsed = start_time.elapsed();
        let ticks = Wrapping(elapsed.as_micros());

        let mut action_happened = false;

        // handle opcode timer
        if ticks >= self.next_opcode_time {
            self.handle_opcode(ticks);
            action_happened = true;
        }

        // handle update timer
        if ticks >= self.next_update_time {
            self.handle_update(ticks);
            action_happened = true;
        }

        if action_happened {
            self.do_sleep(ticks);
        }
    }

    fn handle_opcode(&mut self, ticks: Wrapping<u128>) {
        self.keyboard_device.read_keys();

        self.process_opcode();

        self.next_opcode_time = ticks + Wrapping(self.opcode_ticks);
    }

    fn handle_update(&mut self, ticks: Wrapping<u128>) {
        // check events
        self.check_exit();

        // as we are working in milliseconds and our update time is 16.6666667 we increment 16 once and increment 17 twice
        self.dec_delay_timer();
        self.dec_sound_timer();

        // draw to screen
        self.video_device.render();

        // set the beep
        self.audio_device.set_beep(self.sound_timer > 0);

        self.next_update_time = ticks + Wrapping(UPDATE_TICKS);
    }

    // calculate the time till the next action, be it opcode processing or
    // update.
    // sleep until then
    fn do_sleep(&self, ticks: Wrapping<u128>) {
        let mut sleep_time: Wrapping<u128>;
        if self.next_opcode_time < self.next_update_time {
            sleep_time = self.next_opcode_time - ticks;
        } else {
            sleep_time = self.next_update_time - ticks;
        }

        // take 10% off
        sleep_time = Wrapping(((sleep_time.0 as f64) * 0.9) as u128);

        std::thread::sleep(std::time::Duration::from_micros(sleep_time.0 as u64));
    }

    fn check_exit(&self) {
        for event in self.sdl_context.event_pump().unwrap().poll_iter() {
            match event {
                Event::Quit { .. } => {
                    std::process::exit(0);
                }
                _ => {
                    //println!("Another Event!");
                }
            }
        }
    }

    fn process_opcode(&mut self) {
        // we do some weird shit to deal with endian-ness
        let op1 = self.memory[self.pc as usize] as u16;
        let op2 = self.memory[(self.pc + 1) as usize] as u16;

        self.inc_pc();

        let opcode = (op1 << 8) | op2;

        let a = ((opcode >> 12) & 0xF) as u8;
        let x = ((opcode >> 8) & 0xF) as usize;
        let y = ((opcode >> 4) & 0xF) as usize;
        let n = (opcode & 0xF) as u8;

        let nn = (opcode & 0xFF) as u8;
        let nnn = (opcode & 0xFFF) as usize;

        match a {
            0x0 => match nnn {
                0x0E0 => {
                    self.disp_clear();
                }
                0x0EE => {
                    self.flow_return();
                }
                _ => {
                    self.call_machine_code_routine(nnn);
                }
            },
            0x1 => {
                self.flow_goto(nnn);
            }
            0x2 => {
                self.flow_call_subroutine(nnn);
            }
            0x3 => {
                self.cond_if_vx_nn_eq_skip(x, nn);
            }
            0x4 => {
                self.cond_if_vx_nn_neq_skip(x, nn);
            }
            0x5 => {
                self.cond_if_vx_vy_eq_skip(x, y);
            }
            0x6 => {
                self.const_set_vx_nn(x, nn);
            }
            0x7 => {
                self.const_set_add_vx_nn(x, nn);
            }
            0x8 => match n {
                0x0 => {
                    self.assig_vx_to_vy(x, y);
                }
                0x1 => {
                    self.bitop_vx_oreq_vy(x, y);
                }
                0x2 => {
                    self.bitop_vx_andeq_vy(x, y);
                }
                0x3 => {
                    self.bitop_vx_xoreq_vy(x, y);
                }
                0x4 => {
                    self.math_vx_pleq_vy(x, y);
                }
                0x5 => {
                    self.math_vx_mieq_vy(x, y);
                }
                0x6 => {
                    self.bitop_vx_rsh(x);
                }
                0x7 => {
                    self.math_vx_eq_vy_mi_vx(x, y);
                }
                0xE => {
                    self.bitop_vx_lsh(x);
                }
                _ => {
                    self.invalid_opcode_panic();
                }
            },
            0x9 => {
                self.cond_if_vx_vy_neq_skip(x, y);
            }
            0xA => {
                self.mem_set_i(nnn);
            }
            0xB => {
                self.flow_jump_v0_pl(nnn);
            }
            0xC => {
                self.rand_vx_rand_and_nn(x, nn);
            }
            0xD => {
                self.display_draw(x, y, n);
            }
            0xE => match nn {
                0x9E => {
                    self.keyop_if_vx_pressed_skip(x);
                }
                0xA1 => {
                    self.keyop_if_vx_not_pressed_skip(x);
                }
                _ => {
                    self.invalid_opcode_panic();
                }
            },
            0xF => match nn {
                0x07 => {
                    self.timer_set_vx_delay(x);
                }
                0x0A => {
                    self.keyop_vx_set_key(x);
                }
                0x15 => {
                    self.timer_set_delay_vx(x);
                }
                0x18 => {
                    self.sound_set_timer_vx(x);
                }
                0x1E => {
                    self.mem_i_pleq_vx(x);
                }
                0x29 => {
                    self.mem_set_i_sprite_addr_vx(x);
                }
                0x33 => {
                    self.bcd_set_i_vx(x);
                }
                0x55 => {
                    self.mem_reg_dump(x);
                }
                0x65 => {
                    self.mem_reg_load(x);
                }
                _ => {
                    self.invalid_opcode_panic();
                }
            },
            _ => {
                self.invalid_opcode_panic();
            }
        }
    }

    fn invalid_opcode_panic(&self) {
        panic!("Invalid opcode")
    }

    // decrement the delay timer if delay timer is not 0
    fn dec_delay_timer(&mut self) {
        if self.delay_timer != 0 {
            self.delay_timer -= 1;
        }
    }

    // decrement the sound timer if sounds timer is not 0
    fn dec_sound_timer(&mut self) {
        if self.sound_timer != 0 {
            self.sound_timer -= 1;
        }
    }

    // go the next instruction - as instructions are 2 bytes long that means
    // moving the program counter along by 2
    fn inc_pc(&mut self) {
        self.pc = self.pc + 2;
    }

    // go to the previous instruction - as instruction are 2 bytes long that
    // means moving theh program counter back by 2
    fn dec_pc(&mut self) {
        self.pc = self.pc - 2;
    }

    fn cond_inc_pc(&mut self, val: bool) {
        if val {
            self.inc_pc();
        }
    }

    // add rhs to lhs
    // if lhs + rhs > u8::max then set VF to 1 else set VF to 0
    fn add_with_carry(&mut self, lhs: Wrapping<u8>, rhs: Wrapping<u8>) -> Wrapping<u8> {
        let max = Wrapping(u8::MAX);
        self.set_carry((max - lhs) < rhs);
        lhs + rhs
    }

    // subtract rhs from lhs.
    // if lhs < rhs then set VF to 1 else set VF to 0
    fn subtract_with_borrow(&mut self, lhs: Wrapping<u8>, rhs: Wrapping<u8>) -> Wrapping<u8> {
        self.set_carry(lhs >= rhs);
        lhs - rhs
    }

    // if val == true set VF=1 else VF=0
    fn set_carry(&mut self, val: bool) {
        if val {
            self.registers[0xF] = Wrapping(1);
        } else {
            self.registers[0xF] = Wrapping(0);
        }
    }

    // push the 12 bit memory address to the stack and increment the
    // stack pointer
    // if no more space on the stack then panic!()
    fn push_stack(&mut self, addr: usize) {
        if self.sp == STACK_SLOTS - 1 {
            panic!("Stack overflow");
        }

        self.stack.push(addr);
        self.sp += 1;
    }

    // pop the 12 bit memory address from the stack and decrement the stack
    // pointer
    // if nothing is on the stack then panic!()
    fn pop_stack(&mut self) -> usize {
        if self.sp == 0 {
            panic!("Stack underflow");
        }

        let addr = self.stack.pop().unwrap();
        self.sp -= 1;
        addr
    }

    // xor the pixel at the coordinate
    // return true if pixel was set from 1 to 0 (collision)
    fn xor_display_pixel(&mut self, x: u8, y: u8, val: u8) -> bool {
        let pixel_bit_cur = self.video_device.get_pixel(x, y);
        self.video_device.set_pixel(x, y, val);

        // collision happened
        return (val == 1) && pixel_bit_cur != val;
    }

    // xor the row of pixels starting at coordinate x,y with pixels defined in
    // row_val
    // return true if any pixel in the row was set from 1 to 0 (collision)
    fn xor_display_row(&mut self, x: u8, y: u8, row_val: u8) -> bool {
        let mut output = false;
        for i in 0..8 {
            // if wrap to other side of screen happens then skip
            let xpixel = (x + i) as usize;
            if xpixel == self.video_device.get_width() {
                break;
            } else if self.xor_display_pixel(xpixel as u8, y, row_val >> (7 - i)) {
                output = true;
            }
        }
        output
    }

    // functions to process opcodes
    // Call machine code routine at addres NNN
    // Op code: 0NNN
    fn call_machine_code_routine(&mut self, _addr: usize) {
        panic!("Not Implemented");
    }

    // Clear the screen
    // Op code: 00E0
    fn disp_clear(&mut self) {
        self.video_device.clear();
    }

    // return from a subroutine
    // Op code: 00EE
    fn flow_return(&mut self) {
        if self.sp > 0 {
            self.pc = self.pop_stack();
        }
    }

    // Jump to the addr at NNN
    // Op code: 1NNN
    fn flow_goto(&mut self, addr: usize) {
        self.pc = addr;
    }

    // Call subroutine at NNN
    // Op code: 2NNN
    fn flow_call_subroutine(&mut self, addr: usize) {
        self.push_stack(self.pc);
        self.pc = addr;
    }

    // Skip the next instruction if VX eq NN
    // Op code: 3XNN
    fn cond_if_vx_nn_eq_skip(&mut self, vxindex: usize, val: u8) {
        self.cond_inc_pc(self.registers[vxindex].0 == val);
    }

    // Skip the next instruction if VX neq NN
    // Op code: 4XNN
    fn cond_if_vx_nn_neq_skip(&mut self, vxindex: usize, val: u8) {
        self.cond_inc_pc(self.registers[vxindex].0 != val);
    }

    // Skip the next instruction if VX eq VY
    // Op code: 5XY0
    fn cond_if_vx_vy_eq_skip(&mut self, vxindex: usize, vyindex: usize) {
        self.cond_inc_pc(self.registers[vxindex].0 == self.registers[vyindex].0);
    }

    // Set VX to NN
    // Op code: 6XNN
    fn const_set_vx_nn(&mut self, vxindex: usize, val: u8) {
        self.registers[vxindex] = Wrapping(val);
    }

    // Add NN to VX (Carry flag not changed)
    // Op code: 7XNN
    fn const_set_add_vx_nn(&mut self, vxindex: usize, val: u8) {
        self.registers[vxindex] += Wrapping(val);
    }

    // Assign VY to VX
    // Op code: 8XY0
    fn assig_vx_to_vy(&mut self, vxindex: usize, vyindex: usize) {
        self.registers[vxindex] = self.registers[vyindex];
    }

    // Set VX to VX or VY
    // Op code: 8XY1
    fn bitop_vx_oreq_vy(&mut self, vxindex: usize, vyindex: usize) {
        self.registers[vxindex] |= self.registers[vyindex];
    }

    // Set VX to VX and VY
    // Op code: 8XY2
    fn bitop_vx_andeq_vy(&mut self, vxindex: usize, vyindex: usize) {
        self.registers[vxindex] &= self.registers[vyindex];
    }

    // Set VX to VX xor VY
    // Op code: 8XY3
    fn bitop_vx_xoreq_vy(&mut self, vxindex: usize, vyindex: usize) {
        self.registers[vxindex] ^= self.registers[vyindex];
    }

    // Set VX to VX plus VY
    // Op code: 8XY4
    fn math_vx_pleq_vy(&mut self, vxindex: usize, vyindex: usize) {
        self.registers[vxindex] =
            self.add_with_carry(self.registers[vxindex], self.registers[vyindex]);
    }

    // Set VX to VX minus VY
    // Op code: 8XY5
    fn math_vx_mieq_vy(&mut self, vxindex: usize, vyindex: usize) {
        self.registers[vxindex] =
            self.subtract_with_borrow(self.registers[vxindex], self.registers[vyindex]);
    }

    // Store least significant bit of VX in VF then right shift VX
    // Op code: 8XY6
    fn bitop_vx_rsh(&mut self, vxindex: usize) {
        self.registers[0xF] = Wrapping(self.registers[vxindex].0 & 0x1);
        self.registers[vxindex] >>= 1;
    }

    // Set VX to VY minus VX
    // Op code: 8XY7
    fn math_vx_eq_vy_mi_vx(&mut self, vxindex: usize, vyindex: usize) {
        self.registers[vxindex] =
            self.subtract_with_borrow(self.registers[vyindex], self.registers[vxindex]);
    }

    // Store most significant bit of VX in VF then left shift VX
    // Op code: 8XYE
    fn bitop_vx_lsh(&mut self, vxindex: usize) {
        self.registers[0xF] = Wrapping((self.registers[vxindex].0 >> 7) & 0x1);
        self.registers[vxindex] <<= 1;
    }

    // Skip the next instruction if VX neq VY
    // Op code: 9XY0
    fn cond_if_vx_vy_neq_skip(&mut self, vxindex: usize, vyindex: usize) {
        self.cond_inc_pc(self.registers[vxindex] != self.registers[vyindex]);
    }

    // Set I to NNN
    // Op code: ANNN
    fn mem_set_i(&mut self, addr: usize) {
        self.i = addr;
    }

    // Jump to the address V0 + NNN
    // Op code: BNNN
    fn flow_jump_v0_pl(&mut self, addr: usize) {
        self.pc = (self.registers[0].0 as usize) + addr;
    }

    // Set VX to rand() and NN
    // Op code: CXNN
    fn rand_vx_rand_and_nn(&mut self, vxindex: usize, val: u8) {
        let random_val: u8 = rand::thread_rng().gen();
        self.registers[vxindex] = Wrapping(random_val & val);
    }

    // Draw a sprite at coordinate VX, VY with width 8: height: N
    // Pixels are read from memory location I. I remains unchanged
    // VF set to one if any screen pixels are unset due to xor or 0 if not
    // Op code: DXYN
    fn display_draw(&mut self, vxindex: usize, vyindex: usize, height: u8) {
        let vx = self.registers[vxindex].0;
        let vy = self.registers[vyindex].0;
        let mut carry = false;
        for i in 0..height as usize {
            let row_index = vy as usize + i;
            if row_index < self.video_device.get_height() {
                if self.xor_display_row(vx, row_index as u8, self.memory[self.i + i]) {
                    carry = true;
                }
            } else {
                break;
            }
        }

        self.set_carry(carry);
    }

    // Skip the next instruction if key at VX is pressed
    // Op code: EX9E
    fn keyop_if_vx_pressed_skip(&mut self, vxindex: usize) {
        let vx = self.registers[vxindex];
        let key_pressed = self.keyboard_device.is_key_pressed(vx.0);

        if key_pressed {
            self.inc_pc();
        }

        self.keyboard_device.clear_keys();
    }

    // Skip the next is instruction if key at VX is not pressed
    // Op code: EXA1
    fn keyop_if_vx_not_pressed_skip(&mut self, vxindex: usize) {
        let vx = self.registers[vxindex];
        let key_pressed = self.keyboard_device.is_key_pressed(vx.0);

        if !key_pressed {
            self.inc_pc();
        }

        self.keyboard_device.clear_keys();
    }

    // Set VX to the value of the delay timer
    // Op code: FX07
    fn timer_set_vx_delay(&mut self, vxindex: usize) {
        self.registers[vxindex] = Wrapping(self.delay_timer);
    }

    // Set VX to next key press - blocking operation until key is pressed
    // Op code: FX0A
    fn keyop_vx_set_key(&mut self, vxindex: usize) {
        match self.keyboard_device.get_key_press() {
            Some(key) => {
                self.registers[vxindex] = Wrapping(key);
                self.keyboard_device.clear_keys();
            }
            None => {
                self.dec_pc();
            }
        }
    }

    // Set the delay timer to VX
    // Op code: FX15
    fn timer_set_delay_vx(&mut self, vxindex: usize) {
        self.delay_timer = self.registers[vxindex].0;
    }

    // Set the sound timer to VX
    // Op code: FX18
    fn sound_set_timer_vx(&mut self, vxindex: usize) {
        self.sound_timer = self.registers[vxindex].0;
    }

    // Add VX to I. VF is not affected
    // Op code: FX1E
    fn mem_i_pleq_vx(&mut self, vxindex: usize) {
        self.i += self.registers[vxindex].0 as usize;
    }

    // Set I to the location of the sprite for the character in VX
    // Op code: FX29
    fn mem_set_i_sprite_addr_vx(&mut self, vxindex: usize) {
        let vx = self.registers[vxindex].0;
        self.i = FONT_START + (FONT_CHAR_SIZE * vx as usize);
    }

    // Store the binary-coded decimal repsentation of VX to the location at I
    // *(I+0) = BCD(3) -> VX hundreds
    // *(I+1) = BCD(2) -> VX tens
    // *(I+2) = BCD(1) -> VX ones
    // Op code: FX33
    fn bcd_set_i_vx(&mut self, vxindex: usize) {
        let mut vx = self.registers[vxindex].0;

        for i in (0..3).rev() {
            self.memory[self.i + i] = vx % 10;
            vx /= 10;
        }
    }

    // Store from V0 to VX to memory starting at I. I remains unchanged
    // Op code: FX55
    fn mem_reg_dump(&mut self, vxindex: usize) {
        for i in 0..vxindex + 1 {
            self.memory[self.i + i] = self.registers[i].0;
        }
    }

    // Load from I to V0 through VX. I remains unchaged
    // Op code: FX65
    fn mem_reg_load(&mut self, vxindex: usize) {
        for i in 0..vxindex + 1 {
            self.registers[i] = Wrapping(self.memory[self.i + i]);
        }
    }
}
