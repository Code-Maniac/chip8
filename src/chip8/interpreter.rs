extern crate rand;
extern crate sdl2;

use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::keyboard::Scancode;
use sdl2::Sdl;
use std::fs;
use std::num::Wrapping;
use std::path::Path;
use std::time::Instant;

// chip 8 has 0x1000 bytes of memory and therefore uses 12 bit addressing
const ADDRESS_SIZE_BITS: usize = 12;

// define constants for using the memory
// Chip 8 has 4096 bytes
const MEM_SIZE: usize = 0x1000;
// 96 bytes for the stack
const STACK_SIZE: usize = 0x60;
// 256 bytes for the display
const DISPLAY_WIDTH: usize = 64;
const DISPLAY_WIDTH_BYTES: usize = DISPLAY_WIDTH / 8;
const DISPLAY_HEIGHT: usize = 32;
const DISPLAY_HEIGHT_BYTES: usize = DISPLAY_HEIGHT / 8;

const DISPLAY_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT;
const DISPLAY_SIZE_BYTES: usize = DISPLAY_SIZE / 8;

// 16 registers
const REGISTERS_SIZE: usize = 0x10;
const TICK_INCREMENTS: [u32; 3] = [16, 17, 17];

const PROGRAM_START: usize = 0x200;
const STACK_START: usize = 0xEA0;
const DISPLAY_START: usize = 0xF00;

const STACK_SLOTS: usize = (STACK_SIZE * 8) / ADDRESS_SIZE_BITS;

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

pub struct Interpreter {
    // the sdl context used for drawing and key polling
    sdl_context: Sdl,

    // the memory
    memory: [u8; MEM_SIZE],

    // 16 variables variables
    registers: [Wrapping<u8>; REGISTERS_SIZE],

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

    // program timer
    next_update_time: Wrapping<u32>,

    // timer decrement timer
    next_timer_dec_time: Wrapping<u32>,

    // the index of the ticks to use for the next update
    tick_increment_index: usize,

    // counter +1 for every time update is called
    update_counter: usize,
}

impl Interpreter {
    pub fn load(romfile: &Path) -> Result<Interpreter, &'static str> {
        let sdl_context = sdl2::init().unwrap();

        let mut interp = Interpreter {
            sdl_context,
            memory: [0; MEM_SIZE],
            registers: [Wrapping(0); REGISTERS_SIZE],
            sp: 0,
            pc: PROGRAM_START,
            i: 0,
            delay_timer: 0,
            sound_timer: 0,
            next_update_time: Wrapping(0u32),
            next_timer_dec_time: Wrapping(0u32),
            tick_increment_index: 0,
            update_counter: 0,
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
        let ticks = elapsed.as_millis() as u32;

        if ticks >= self.next_update_time.0 {
            self.process_opcode();
            self.update_counter += 1;

            // as we are working in milliseconds and our update time is 16.6666667 we increment 16 once and increment 17 twice
            let inc = TICK_INCREMENTS[self.tick_increment_index];
            self.next_update_time += Wrapping(inc);
            self.tick_increment_index = self.tick_increment_index % TICK_INCREMENTS.len();
        }

        if ticks >= self.next_timer_dec_time.0 {
            self.dec_delay_timer();
            self.dec_sound_timer();
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

        println!("Opcode: {:#04X}", opcode);
        println!("a: {:#X}", a);
        println!("x: {:#X}", x);
        println!("y: {:#X}", y);
        println!("n: {:#X}", n);

        println!("nn: {:#02X}", nn);
        println!("nnn: {:#03X}", nnn);

        println!("pc: {}\tsp: {}", self.pc, self.sp);
        println!("Loops: {}\n", self.update_counter);

        if opcode == 0x00E0 {
            self.disp_clear();
        } else if opcode == 0x00EE {
            self.flow_return();
        } else if a == 0 {
            self.call_machine_code_routine(nnn);
        } else if a == 0x1 {
            self.flow_goto(nnn);
        } else if a == 0x2 {
            self.flow_call_subroutine(nnn);
        } else if a == 0x3 {
            self.cond_if_vx_nn_eq_skip(x, nn);
        } else if a == 0x4 {
            self.cond_if_vx_nn_neq_skip(x, nn);
        } else if a == 0x5 {
            self.cond_if_vx_vy_eq_skip(x, y);
        } else if a == 0x6 {
            self.const_set_vx_nn(x, nn);
        } else if a == 0x7 {
            self.const_set_add_vx_nn(x, nn);
        } else if a == 0x8 {
            if n == 0x0 {
                self.assig_vx_to_vy(x, y);
            } else if n == 0x1 {
                self.bitop_vx_oreq_vy(x, y);
            } else if n == 0x2 {
                self.bitop_vx_andeq_vy(x, y);
            } else if n == 0x3 {
                self.bitop_vx_xoreq_vy(x, y);
            } else if n == 0x4 {
                self.math_vx_pleq_vy(x, y);
            } else if n == 0x5 {
                self.math_vx_mieq_vy(x, y);
            } else if n == 0x6 {
                self.bitop_vx_rsh(x);
            } else if n == 0x7 {
                self.math_vx_eq_vy_mi_vx(x, y);
            } else if n == 0xE {
                self.bitop_vx_lsh(x);
            } else {
                panic!();
            }
        } else if a == 0x9 {
            self.cond_if_vx_vy_neq_skip(x, y);
        } else if a == 0xA {
            self.mem_set_i(nnn);
        } else if a == 0xB {
            self.flow_jump_v0_pl(nnn);
        } else if a == 0xC {
            self.rand_vx_rand_and_nn(x, nn);
        } else if a == 0xD {
            self.display_draw(x, y, n);
        } else if a == 0xE {
            if nn == 0x9E {
                self.keyop_if_vx_pressed_skip(x);
            } else if nn == 0xA1 {
                self.keyop_if_vx_not_pressed_skip(x);
            } else {
                panic!();
            }
        } else if a == 0xF {
            if nn == 0x07 {
                self.timer_set_vx_delay(x);
            } else if nn == 0x0A {
                self.keyop_vx_set_key(x);
            } else if nn == 0x15 {
                self.timer_set_delay_vx(x);
            } else if nn == 0x18 {
                self.sound_set_timer_vx(x);
            } else if nn == 0x1E {
                self.mem_i_pleq_vx(x);
            } else if nn == 0x29 {
                self.mem_set_i_sprite_addr_vx(x);
            } else if nn == 0x33 {
                self.bcd_set_i_vx(x);
            } else if nn == 0x55 {
                self.mem_reg_dump(x);
            } else if nn == 0x65 {
                self.mem_reg_load(x);
            } else {
                panic!();
            }
        } else {
            panic!();
        }
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

    fn cond_inc_pc(&mut self, val: bool) {
        if val {
            self.inc_pc();
        }
    }

    fn do_carry(&mut self, new_val: Wrapping<u8>, old_val: u8) {
        if new_val.0 < old_val {
            self.registers[0xF] = Wrapping(1);
        } else {
            self.registers[0xF] = Wrapping(0);
        }
    }

    // push the 12 bit memory address to the stack and increment the
    // stack pointer
    // if no more space on the stack then panic!()
    fn push_stack(&mut self, addr: usize) {
        self.sp += 1;
        if self.sp == STACK_SLOTS {
            panic!("Stack overflow");
        }

        // calculate the position in bits
        let bit_position = self.sp * ADDRESS_SIZE_BITS;

        let bytes_position = bit_position / 8;
        let bits = bit_position % 8;

        let stack_pos = STACK_START + bytes_position;
        if bits == 0 {
            // we are aligned with a byte
            self.memory[stack_pos] = addr as u8;
            self.memory[stack_pos + 1] = ((addr >> 8) & 0xF) as u8;
        } else {
            // we are 4 bits out of phase with a byte
            self.memory[stack_pos] &= ((addr << 4) & 0xF0) as u8;
            self.memory[stack_pos] = (addr >> 8) as u8;
        }
    }

    // pop the 12 bit memory address from the stack and decrement the stack
    // pointer
    // if nothing is on the stack then panic!()
    fn pop_stack(&mut self) -> usize {
        if self.sp == 0 {
            panic!("Stack underflow");
        }

        // calculate the position in bits
        let bit_position = self.sp * ADDRESS_SIZE_BITS;

        // decrement the stack pointer
        self.sp -= 1;

        let bytes_position = bit_position / 8;
        let bits = bit_position % 8;

        let stack_pos = STACK_START + bytes_position;
        if bits == 0 {
            // we are aligned with a byte
            let addr1 = self.memory[stack_pos] as usize;
            let addr2 = ((self.memory[stack_pos + 1] as usize) & 0xF) << 8;
            addr1 | addr2
        } else {
            // we are 4 bits out of phase with a byte
            let addr1 = (self.memory[stack_pos] >> 4) as usize;
            let addr2 = (self.memory[stack_pos + 1] as usize) << 8;
            addr1 | addr2
        }
    }

    // get the value of the pixel at the coordinate (0 or 1)
    fn get_display_pixel(&self, x: u8, y: u8) -> u8 {
        // get the pixel value at coordinate x, y
        //let start_bit = (y as usize * DISPLAY_WIDTH) + (x % DISPLAY_WIDTH) as usize;

        panic!("Not Implemented");
    }

    // xor the pixel at the coordinate
    // return true if pixel was set from 1 to 0 (collision)
    fn xor_display_pixel(&mut self, x: u8, y: u8, val: u8) -> bool {
        let current_bit_val = self.get_display_pixel(x, y);

        // collision happen
        return (val == 1) && current_bit_val != val;
    }

    // xor the row of pixels starting at coordinate x,y with pixels defined in
    // row_val
    // return true if any pixel in the row was set from 1 to 0 (collision)
    fn xor_display_row(&mut self, x: u8, y: u8, row_val: u8) -> bool {
        let mut output = false;
        for i in x..x + 8 {
            if self.xor_display_pixel(i, y, row_val >> i) {
                output = true;
            }
        }
        output
    }

    // functions to process opcodes
    // Call machine code routine at addres NNN
    // Op code: 0NNN
    fn call_machine_code_routine(&mut self, addr: usize) {
        panic!("Not Implemented");
    }

    // Clear the screen
    // Op code: 00E0
    fn disp_clear(&mut self) {
        // set all pixels to 0
        for i in DISPLAY_START..DISPLAY_START + DISPLAY_SIZE_BYTES {
            self.memory[i] = 0;
        }
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
        self.sp = self.sp + 1;
        if self.sp as usize == STACK_SIZE {
            panic!("Stack overflow");
        } else {
            self.pc = addr;
            self.push_stack(self.pc);
        }
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
        self.cond_inc_pc(self.registers[vxindex].0 != self.registers[vyindex].0);
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
        let tmp = self.registers[vxindex].0;

        self.registers[vxindex] += self.registers[vyindex];

        self.do_carry(self.registers[vxindex], tmp);
    }

    // Set VX to VX minus VY
    // Op code: 8XY5
    fn math_vx_mieq_vy(&mut self, vxindex: usize, vyindex: usize) {
        let tmp = self.registers[vxindex].0;

        self.registers[vxindex] -= self.registers[vyindex];

        self.do_carry(self.registers[vxindex], tmp);
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
        let vx = self.registers[vxindex];
        self.registers[vxindex] = self.registers[vyindex] - vx;
        self.do_carry(self.registers[vxindex], vx.0);
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
        panic!("Not Implemented");
    }

    // Skip the next instruction if key at VX is pressed
    // Op code: EX9E
    fn keyop_if_vx_pressed_skip(&mut self, vxindex: usize) {
        let vx = self.registers[vxindex];
        let code = SCAN_CODES[vx.0 as usize];

        if self
            .sdl_context
            .event_pump()
            .unwrap()
            .keyboard_state()
            .is_scancode_pressed(code)
        {
            self.inc_pc();
        }
    }

    // Skip the next is instruction if key at VX is not pressed
    // Op code: EXA1
    fn keyop_if_vx_not_pressed_skip(&mut self, vxindex: usize) {
        let vx = self.registers[vxindex];
        let code = SCAN_CODES[vx.0 as usize];

        if !self
            .sdl_context
            .event_pump()
            .unwrap()
            .keyboard_state()
            .is_scancode_pressed(code)
        {
            self.inc_pc();
        }
    }

    // Set VX to the value of the delay timer
    // Op code: FX07
    fn timer_set_vx_delay(&mut self, vxindex: usize) {
        self.registers[vxindex] = Wrapping(self.delay_timer);
    }

    // Set VX to next key press - blocking operation until key is pressed
    // Op code: FX0A
    fn keyop_vx_set_key(&mut self, vxindex: usize) {
        let mut key: u8 = 0xFF;

        // now keep polling events until a key down happens on:
        // REAL -> CHIP8
        // 1234 -> 123B
        // qwer -> 456C
        // asdf -> 789D
        // zxcv -> A0BF
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

        self.registers[vxindex] = Wrapping(key);
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

        for i in (0..2).rev() {
            self.memory[self.i + i] = vx % 10;
            vx /= 10;
        }
    }

    // Store from V0 to VX to memory starting at I. I remains unchanged
    // Op code: FX55
    fn mem_reg_dump(&mut self, vxindex: usize) {
        for i in 0..vxindex {
            self.memory[self.i + i] = self.registers[i].0;
        }
    }

    // Load from I to V0 through VX. I remains unchaged
    // Op code: FX65
    fn mem_reg_load(&mut self, vxindex: usize) {
        for i in 0..vxindex {
            self.registers[i] = Wrapping(self.memory[self.i + i]);
        }
    }
}
