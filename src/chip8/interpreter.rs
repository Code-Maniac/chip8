use std::fs;
use std::path::Path;
use std::time::Instant;

const TICK_INCREMENTS: [u32; 3] = [16, 17, 17];

const REGISTERS_SIZE: usize = 0x10;
const MEM_SIZE: usize = 0x2000;
const STACK_SIZE: usize = 0x100;

pub struct Interpreter {
    // the loaded chip8 rom data
    data: Vec<u8>,

    // the memory
    memory: [u8; MEM_SIZE],

    // 16 variables variables
    registers: [u8; REGISTERS_SIZE],

    // stack for subroutines
    // I guess we can give as many levels of nesting as we want
    stack: [u16; STACK_SIZE],

    // the stack pointer
    sp: u16,

    // Program counter - represents the current position in execution of the
    // program
    pc: u16,

    // 16 bit register for memory addresses
    i: u16,

    // timers
    delay_timer: u8,
    sound_timer: u8,

    // program timer
    next_update_time: u32,

    // the index of the ticks to use for the next update
    tick_increment_index: usize,
}

impl Interpreter {
    pub fn load(romfile: &Path) -> Result<Interpreter, &'static str> {
        let data = fs::read(romfile).expect("Could not load romfile");
        Ok(Interpreter {
            data,
            memory: [0; MEM_SIZE],
            registers: [0; REGISTERS_SIZE],
            stack: [0; STACK_SIZE],
            sp: 0,
            pc: 0,
            i: 0,
            delay_timer: 0,
            sound_timer: 0,
            next_update_time: 0,
            tick_increment_index: 0,
        })
    }

    // function to do next cpu cycle
    pub fn update(&mut self, start_time: &Instant) {
        let elapsed = start_time.elapsed();
        let ticks = elapsed.as_millis() as u32;

        if ticks >= self.next_update_time {
            self.process_opcode();
        }

        // as we are working in milliseconds and our update time is 16.6666667
        // we increment 16 once and increment 17 twice
        let inc = TICK_INCREMENTS[self.tick_increment_index];
        self.next_update_time = self.next_update_time + TICK_INCREMENTS[self.tick_increment_index];
        self.tick_increment_index = self.tick_increment_index % TICK_INCREMENTS.len()
    }

    pub fn render(&self) {
        // draw the current graphics array to the screen
    }

    fn process_opcode(&mut self) {
        let op1 = self.data[self.pc as usize] as u16;
        let op2 = self.data[self.pc as usize] as u16;

        let opcode = (op1 << 8) | op2;

        // unpack opcode
        let a = (opcode >> 24) & 0xF;
        let x = (opcode >> 16) & 0xF;
        let y = (opcode >> 8) & 0xF;
        let n = (opcode & 0x000F) as u8;

        let nn = (opcode & 0xFF) as u8;
        let nnn = opcode & 0x0FFF;

        if opcode == 0x00E0 {
            self.disp_clear();
        } else if opcode == 0x00EE {
            self.flow_return();
        } else if a == 0 {
            self.call_machine_code_routine(opcode & 0xFFF);
        } else if a == 0x1 {
            self.flow_goto(nnn);
        } else if a == 0x2 {
            self.flow_call_subroutine(nnn);
        } else if a == 0x3 {
            self.cond_if_vx_nn_eq_skip(x as usize, nn);
        } else if a == 0x4 {
            self.cond_if_vx_nn_neq_skip(x as usize, nn);
        } else if a == 0x5 {
            self.cond_if_vx_vy_eq_skip(x as usize, y as usize);
        } else if a == 0x6 {
            self.const_set_vx_nn(x as usize, nn);
        } else if a == 0x7 {
            self.const_set_add_vx_nn(x as usize, nn);
        } else if a == 0x8 && n == 0x0 {
            if n == 0x0 {
                self.assig_vx_to_vy(x as usize, y as usize);
            } else if n == 0x1 {
                self.bitop_vx_oreq_vy(x as usize, y as usize);
            } else if n == 0x2 {
                self.bitop_vx_andeq_vy(x as usize, y as usize);
            } else if n == 0x3 {
                self.bitop_vx_xoreq_vy(x as usize, y as usize);
            } else if n == 0x4 {
                self.math_vx_pleq_vy(x as usize, y as usize);
            } else if n == 0x5 {
                self.math_vx_mieq_vy(x as usize, y as usize);
            } else if n == 0x6 {
                self.bitop_vx_rsh(x as usize);
            } else if n == 0x7 {
                self.math_vx_eq_vy_mi_vx(x as usize, y as usize);
            } else if n == 0xE {
                self.bitop_vx_lsh(x as usize);
            } else {
                panic!();
            }
        } else if a == 0x9 {
            self.cond_if_vx_vy_eq_skip(x as usize, y as usize);
        } else if a == 0xA {
            self.mem_set_i(nnn);
        } else if a == 0xB {
            self.flow_jump_v0_pl(nnn);
        } else if a == 0xC {
            self.rand_vx_rand_and_nn(x as usize, nn);
        } else if a == 0xD {
            self.display_draw(x as usize, y as usize, n);
        } else if a == 0xE {
            if nn == 0x9E {
                self.keyop_if_vx_pressed_skip(x as usize);
            } else if nn == 0xA1 {
                self.keyop_if_vx_not_pressed_skip(x as usize);
            } else {
                panic!();
            }
        } else if a == 0xF {
            if nn == 0x07 {
                self.timer_set_vx_delay(x as usize);
            } else if nn == 0x0A {
                self.keyop_vx_set_key(x as usize);
            } else if nn == 0x15 {
                self.timer_set_delay_vx(x as usize);
            } else if nn == 0x18 {
                self.sound_set_timer_vx(x as usize);
            } else if nn == 0x1E {
                self.mem_i_pleq_vx(x as usize);
            } else if nn == 0x29 {
                self.mem_set_i_sprite_addr_vx(x as usize);
            } else if nn == 0x33 {
                self.bcd_set_i_vx(x as usize);
            } else if nn == 0x55 {
                self.mem_reg_dump(x as usize);
            } else if nn == 0x65 {
                self.mem_reg_load(x as usize);
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    }

    // go the next instruction - as instructions are 2 bytes long that means
    // moving the program counter along by 2
    fn inc_pc(&mut self) {
        self.pc = self.pc + 2;
    }

    // skip the next instruction
    fn skip_pc(&mut self) {
        self.pc = self.pc + 4;
    }

    fn inc_or_skip_pc(&mut self, skip: bool) {
        if skip {
            self.skip_pc();
        } else {
            self.inc_pc();
        }
    }

    // functions to process opcodes

    // Call machine code routine at addres NNN
    // Op code: 0NNN
    fn call_machine_code_routine(&self, addr: u16) {
        panic!("Not Implemented");
    }

    // Clear the screen
    // Op code: 00E0
    fn disp_clear(&mut self) {
        panic!("Not Implemented");
    }

    // return from a subroutine
    // Op code: 00EE
    fn flow_return(&mut self) {
        if self.sp > 0 {
            self.sp = self.sp - 1;
            self.pc = self.stack[self.sp as usize];
        }
    }

    // Jump to the addr at NNN
    // Op code: 1NNN
    fn flow_goto(&mut self, addr: u16) {
        self.pc = addr;
    }

    // Call subroutine at NNN
    // Op code: 2NNN
    fn flow_call_subroutine(&mut self, addr: u16) {
        self.sp = self.sp + 1;
        if self.sp as usize == STACK_SIZE {
            panic!("Stack overflow");
        } else {
            self.stack[self.sp as usize] = self.pc;
            self.pc = addr;
        }
    }

    // Skip the next instruction if VX eq NN
    // Op code: 3XNN
    fn cond_if_vx_nn_eq_skip(&mut self, vxindex: usize, val: u8) {
        self.inc_or_skip_pc(self.registers[vxindex] == val);
    }

    // Skip the next instruction if VX neq NN
    // Op code: 4XNN
    fn cond_if_vx_nn_neq_skip(&mut self, vxindex: usize, val: u8) {
        self.inc_or_skip_pc(self.registers[vxindex] != val);
    }

    // Skip the next instruction if VX eq VY
    // Op code: 5XY0
    fn cond_if_vx_vy_eq_skip(&mut self, vxindex: usize, vyindex: usize) {
        self.inc_or_skip_pc(self.registers[vxindex] != self.registers[vyindex]);
    }

    // Set VX to NN
    // Op code: 6XNN
    fn const_set_vx_nn(&mut self, vxindex: usize, val: u8) {
        self.registers[vxindex] = val;
        self.inc_pc();
    }

    // Add NN to VX (Carry flag not changed)
    // Op code: 7XNN
    fn const_set_add_vx_nn(&mut self, vxindex: usize, val: u8) {
        let vx = &self.registers[vxindex];
        vx = &(vx + val);
        self.inc_pc();
    }

    // Assign VY to VX
    // Op code: 8XY0
    fn assig_vx_to_vy(&mut self, vxindex: usize, vyindex: usize) {
        let vx = &self.registers[vxindex];
        vx = &(vx + self.registers[vyindex]);
        self.inc_pc();
    }

    // Set VX to VX or VY
    // Op code: 8XY1
    fn bitop_vx_oreq_vy(&mut self, vxindex: usize, vyindex: usize) {
        let vx = &self.registers[vxindex];
        vx = &(vx | self.registers[vyindex]);
        self.inc_pc();
    }

    // Set VX to VX and VY
    // Op code: 8XY2
    fn bitop_vx_andeq_vy(&mut self, vxindex: usize, vyindex: usize) {
        let vx = &self.registers[vxindex];
        vx = &(vx & self.registers[vyindex]);
        self.inc_pc();
    }

    // Set VX to VX xor VY
    // Op code: 8XY3
    fn bitop_vx_xoreq_vy(&mut self, vxindex: usize, vyindex: usize) {
        let vx = &self.registers[vxindex];
        vx = &(vx ^ self.registers[vyindex]);
        self.inc_pc();
    }

    // Set VX to VX plus VY
    // Op code: 8XY4
    fn math_vx_pleq_vy(&mut self, vxindex: usize, vyindex: usize) {
        let vx = &self.registers[vxindex];
        vx = &(vx + self.registers[vyindex]);
        self.inc_pc();
    }

    // Set VX to VX minus VY
    // Op code: 8XY5
    fn math_vx_mieq_vy(&mut self, vxindex: usize, vyindex: usize) {
        let vx = &self.registers[vxindex];
        vx = &(vx - self.registers[vyindex]);
        self.inc_pc();
    }

    // Store least significant bit of VX in VF then right shift VX
    // Op code: 8XY6
    fn bitop_vx_rsh(&mut self, vxindex: usize) {
        let vx = &self.registers[vxindex];
        vx = &(vx >> 1);
        self.inc_pc();
    }

    // Set VX to VY minus VX
    // Op code: 8XY7
    fn math_vx_eq_vy_mi_vx(&mut self, vxindex: usize, vyindex: usize) {
        let vx = &self.registers[vxindex];
        vx = &(self.registers[vyindex] - vx);
    }

    // Store most significant bit of VX in VF then left shift VX
    // Op code: 8XYE
    fn bitop_vx_lsh(&mut self, vxindex: usize) {
        let vx = &self.registers[vxindex];
        vx = &(vx << 1);
        self.inc_pc();
    }

    // Skip the next instruction if VX neq VY
    // Op code: 9XY0
    fn cond_if_vx_neq_vy_skip(&mut self, vxindex: usize, vyindex: usize) {
        self.inc_or_skip_pc(self.registers[vxindex] != self.registers[vyindex]);
    }

    // Set I to NNN
    // Op code: ANNN
    fn mem_set_i(&mut self, addr: u16) {
        self.i = addr;
        self.inc_pc();
    }

    // Jump to the address V0 + NNN
    // Op code: BNNN
    fn flow_jump_v0_pl(&mut self, addr: u16) {
        self.pc = (self.registers[0] as u16) + addr;
    }

    // Set VX to rand() and NN
    // Op code: CXNN
    fn rand_vx_rand_and_nn(&mut self, vxindex: usize, val: u8) {
        panic!("Not Implemented");
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
        panic!("Not Implemented");
    }

    // Skip the next is instruction if key at VX is not pressed
    // Op code: EXA1
    fn keyop_if_vx_not_pressed_skip(&mut self, vxindex: usize) {
        panic!("Not Implemented");
    }

    // Set VX to the value of the delay timer
    // Op code: FX07
    fn timer_set_vx_delay(&mut self, vxindex: usize) {
        self.registers[vxindex] = self.delay_timer;
        self.inc_pc();
    }

    // Set VX to next key press - blocking operation until key is pressed
    // Op code: FX0A
    fn keyop_vx_set_key(&mut self, vxindex: usize) {
        panic!("Not Implemented");
    }

    // Set the delay timer to VX
    // Op code: FX15
    fn timer_set_delay_vx(&mut self, vxindex: usize) {
        self.delay_timer = self.registers[vxindex];
        self.inc_pc();
    }

    // Set the sound timer to VX
    // Op code: FX18
    fn sound_set_timer_vx(&mut self, vxindex: usize) {
        self.sound_timer = self.registers[vxindex];
        self.inc_pc();
    }

    // Add VX to I. VF is not affected
    // Op code: FX1E
    fn mem_i_pleq_vx(&mut self, vxindex: usize) {
        self.i += self.registers[vxindex] as u16;
        self.inc_pc();
    }

    // Set I to the location of the sprite for the character in VX
    // Op code: FX29
    fn mem_set_i_sprite_addr_vx(&mut self, vxindex: usize) {
        panic!("Not Implemented");
    }

    // Store the binary-coded decimal repsentation of VX to the location at I
    // *(I+0) = BCD(3) -> VX hundreds
    // *(I+1) = BCD(2) -> VX tens
    // *(I+2) = BCD(1) -> VX ones
    // Op code: FX33
    fn bcd_set_i_vx(&mut self, vxindex: usize) {
        // let vx = self.registers[vxindex];
        // self.memory[self.i] = (vx - (vx % 100u8)) / 100u8;
        // self.memory[self.i + 1] = (vx % 100) - (vx % 10);

        panic!("Not Implemented");
    }

    // Store from V0 to VX to memory starting at I. I remains unchanged
    // Op code: FX55
    fn mem_reg_dump(&mut self, vxindex: usize) {
        panic!("Not Implemented");
    }

    // Load from I to V0 through VX. I remains unchaged
    // Op code: FX65
    fn mem_reg_load(&mut self, vxindex: usize) {
        panic!("Not Implemented");
    }
}
