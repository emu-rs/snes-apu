// TODO: This will panic on certain opcodes. The ideas I have for cpu_lang
// will probably fix it; we'll see. For now tho, I'm gonna leave it as-is;
// I can imagine fixing it trivially might lead to performance problems.
// I'll deal with it after the port is finished.

// TODO: This is a placeholder before I start generalizing traits
// from the old code.
use super::apu::Apu;

pub struct Smp<'a> {
    emulator: &'a mut Apu,

    reg_pc: u16,
    reg_a: u8,
    reg_x: u8,
    reg_y: u8,
    reg_sp: u8,

    psw_c: bool,
    psw_z: bool,
    psw_h: bool,
    psw_p: bool,
    psw_v: bool,
    psw_n: bool,
    // TODO: Look up some more behavior for I and B. Can't seem to find much
    // but some instructions DO set them.
    psw_i: bool,
    psw_b: bool,

    cycle_count: i32
}

impl<'a> Smp<'a> {
    pub fn new(emulator: &'a mut Apu) -> Smp<'a> {
        let mut ret = Smp {
            emulator: emulator,

            reg_pc: 0,
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_sp: 0,

            psw_c: false,
            psw_z: false,
            psw_h: false,
            psw_p: false,
            psw_v: false,
            psw_n: false,
            psw_i: false,
            psw_b: false,

            cycle_count: 0
        };
        ret.reset();
        ret
    }

    pub fn reset(&mut self) {
        self.reg_pc = 0xffc0;
        self.reg_a = 0;
        self.reg_x = 0;
        self.reg_y = 0;
        self.reg_sp = 0xef;
        self.set_psw(0x02);
    }

    pub fn set_reg_ya(&mut self, value: u16) {
        self.reg_a = (value & 0xff) as u8;
        self.reg_y = ((value >> 8) & 0xff) as u8;
    }

    pub fn get_reg_ya(&self) -> u16 {
        ((self.reg_y as u16) << 8) | (self.reg_a as u16)
    }

    pub fn set_psw(&mut self, value: u8) {
        self.psw_c = (value & 0x01) != 0;
        self.psw_z = (value & 0x02) != 0;
        self.psw_h = (value & 0x08) != 0;
        self.psw_p = (value & 0x20) != 0;
        self.psw_v = (value & 0x40) != 0;
        self.psw_n = (value & 0x80) != 0;
    }

    pub fn get_psw(&self) -> u8 {
        ((if self.psw_n { 1 } else { 0 }) << 7) |
        ((if self.psw_v { 1 } else { 0 }) << 6) |
        ((if self.psw_p { 1 } else { 0 }) << 5) |
        ((if self.psw_h { 1 } else { 0 }) << 3) |
        ((if self.psw_z { 1 } else { 0 }) << 1) |
        (if self.psw_c { 1 } else { 0 })
    }

    fn is_negative(value: u32) -> bool {
        (value & 0x80) != 0
    }

    fn cycles(&mut self, num_cycles: i32) {
        self.emulator.cpu_cycles_callback(num_cycles);
        self.cycle_count += num_cycles;
    }

    fn read_op(&mut self, addr: u16) -> u8 {
        self.cycles(1);
        self.emulator.read_byte(addr as u32)
    }

    fn write_op(&mut self, addr: u16, value: u8) {
        self.cycles(1);
        self.emulator.write_byte(addr as u32, value);
    }

    fn read_pc_op(&mut self) -> u8 {
        let addr = self.reg_pc;
        self.reg_pc += 1;
        let ret = self.read_op(addr);
        ret
    }

    fn read_sp_op(&mut self) -> u8 {
        self.reg_sp += 1;
        let addr = 0x0100 | (self.reg_sp as u16);
        self.read_op(addr)
    }

    fn write_sp_op(&mut self, value: u8) {
        let addr = 0x0100 | (self.reg_sp as u16);
        self.reg_sp -= 1;
        self.write_op(addr, value);
    }

    fn read_dp_op(&mut self, addr: u8) -> u8 {
        let addr = (if self.psw_p { 0x0100 } else { 0 }) | (addr as u16);
        self.read_op(addr)
    }

    fn write_dp_op(&mut self, addr: u8, value: u8) {
        let addr = (if self.psw_p { 0x0100 } else { 0 }) | (addr as u16);
        self.write_op(addr, value);
    }

    fn set_psw_n_z_op(&mut self, x: u32) {
        self.psw_n = Smp::is_negative(x);
        self.psw_z = x == 0;
    }

    fn adc_op(&mut self, x: u8, y: u8) -> u8 {
        let x = x as u32;
        let y = y as u32;
        let r = x + y + (if self.psw_c { 1 } else { 0 });
        self.psw_n = Smp::is_negative(r);
        self.psw_v = (!(x ^ y) & (x ^ r) & 0x80) != 0;
        self.psw_h = ((x ^ y ^ r) & 0x10) != 0;
        self.psw_z = (r & 0xff) == 0;
        self.psw_c = r > 0xff;
        (r & 0xff) as u8
    }

    fn and_op(&mut self, x: u8, y: u8) -> u8 {
        let ret = x & y;
        self.set_psw_n_z_op(ret as u32);
        ret
    }

    fn asl_op(&mut self, x: u8) -> u8 {
        self.psw_c = Smp::is_negative(x as u32);
        let ret = x << 1;
        self.set_psw_n_z_op(ret as u32);
        ret
    }

    fn cmp_op(&mut self, x: u8, y: u8) -> u8 {
        let r = (x as i32) - (y as i32);
        self.psw_n = (r & 0x80) != 0;
        self.psw_z = (r & 0xff) == 0;
        self.psw_c = r >= 0;
        x
    }

    fn dec_op(&mut self, x: u8) -> u8 {
        let ret = x - 1;
        self.set_psw_n_z_op(ret as u32);
        ret
    }

    fn eor_op(&mut self, x: u8, y: u8) -> u8 {
        let ret = x ^ y;
        self.set_psw_n_z_op(ret as u32);
        ret
    }

    fn inc_op(&mut self, x: u8) -> u8 {
        let ret = x + 1;
        self.set_psw_n_z_op(ret as u32);
        ret
    }

    fn ld_op(&mut self, _: u8, y: u8) -> u8 {
        self.set_psw_n_z_op(y as u32);
        y
    }

    fn lsr_op(&mut self, x: u8) -> u8 {
        self.psw_c = (x & 0x01) != 0;
        let ret = x >> 1;
        self.set_psw_n_z_op(ret as u32);
        ret
    }

    fn or_op(&mut self, x: u8, y: u8) -> u8 {
        let ret = x | y;
        self.set_psw_n_z_op(ret as u32);
        ret
    }

    fn rol_op(&mut self, x: u8) -> u8 {
        let carry = if self.psw_c { 1 } else { 0 };
        self.psw_c = (x & 0x80) != 0;
        let ret = (x << 1) | carry;
        self.set_psw_n_z_op(ret as u32);
        ret
    }

    fn ror_op(&mut self, x: u8) -> u8 {
        let carry = if self.psw_c { 0x80 } else { 0 };
        self.psw_c = (x & 0x01) != 0;
        let ret = carry | (x >> 1);
        self.set_psw_n_z_op(ret as u32);
        ret
    }

    fn sbc_op(&mut self, x: u8, y: u8) -> u8 {
        self.adc_op(x, !y)
    }

    fn st_op(&self, _: u8, y: u8) -> u8 {
        y
    }

    fn adw_op(&mut self, x: u16, y: u16) -> u16 {
        self.psw_c = false;
        let mut ret = self.adc_op(x as u8, y as u8) as u16;
        ret |= (self.adc_op((x >> 8) as u8, (y >> 8) as u8) as u16) << 8;
        self.psw_z = ret == 0;
        ret
    }

    fn cpw_op(&mut self, x: u16, y: u16) -> u16 {
        let ret = (x as i32) - (y as i32);
        self.psw_n = (ret & 0x8000) != 0;
        self.psw_z = (ret as u16) == 0;
        self.psw_c = ret >= 0;
        ret as u16
    }

    fn ldw_op(&mut self, _: u16, y: u16) -> u16 {
        self.psw_n = (y & 0x8000) != 0;
        self.psw_z = y == 0;
        y
    }

    fn sbw_op(&mut self, x: u16, y: u16) -> u16 {
        self.psw_c = true;
        let mut ret = self.sbc_op(x as u8, y as u8) as u16;
        ret |= (self.sbc_op((x >> 8) as u8, (y >> 8) as u8) as u16) << 8;
        self.psw_z = ret == 0;
        ret
    }

    fn adjust_dpw_op(&mut self, x: u16) {
        let mut addr = self.read_pc_op();
        let mut result = (self.read_dp_op(addr) as u16) + x;
        self.write_dp_op(addr, result as u8);
        addr += 1;
        let mut high = (result >> 8) as u8;
        high += self.read_dp_op(addr);
        result = ((high as u16) << 8) | (result & 0xff);
        self.write_dp_op(addr, (result >> 8) as u8);
        self.psw_n = (result & 0x8000) != 0;
        self.psw_z = result == 0;
    }

    fn branch_op(&mut self, cond: bool) {
        let offset = self.read_pc_op();
        if !cond {
            return;
        }
        self.cycles(2);
        // TODO: Some of these casts might not be necessary; there's probably
        // a better way to add a i8 to a u16 with proper signs.
        self.reg_pc = ((self.reg_pc as i16) + ((offset as i8) as i16)) as u16;
    }

    fn branch_bit_op(&mut self, x: u8) {
        let addr = self.read_pc_op();
        let sp = self.read_dp_op(addr);
        let y = self.read_pc_op();
        self.cycles(1);
        if ((sp & (1 << (x >> 5))) != 0) == ((x & 0x10) != 0) {
            return;
        }
        self.cycles(2);
        // TODO: Some of these casts might not be necessary; there's probably
        // a better way to add a i8 to a u16 with proper signs.
        self.reg_pc = ((self.reg_pc as i16) + ((y as i8) as i16)) as u16;
    }

    fn pull_op(&mut self, x: &mut u8) {
        self.cycles(2);
        *x = self.read_sp_op();
    }

    fn push_op(&mut self, x: u8) {
        self.cycles(2);
        self.write_sp_op(x);
    }

    fn set_addr_bit_op(&mut self, opcode: u8) {
        let mut x = self.read_pc_op() as u16;
        x |= (self.read_pc_op() as u16) << 8;
        let bit = x >> 13;
        x &= 0x1fff;
        let mut y = self.read_op(x) as u16;
        match opcode >> 5 {
            0 | 1 => { // orc addr:bit; orc !addr:bit
                self.cycles(1);
                self.psw_c |= ((y & (1 << bit)) != 0) ^ ((opcode & 0x20) != 0);
            }
            2 | 3 => { // and addr:bit; and larrd:bit
                self.psw_c &= ((y & (1 << bit)) != 0) ^ ((opcode & 0x20) != 0);
            }
            4 => { // eor addr:bit
                self.cycles(1);
                self.psw_c ^= (y & (1 << bit)) != 0;
            }
            5 => { // ldc addr:bit
                self.psw_c = (y & (1 << bit)) != 0;
            }
            6 => { // stc addr:bit
                self.cycles(1);
                y = (y & !(1 << bit)) | ((if self.psw_c { 1 } else { 0 }) << bit);
                self.write_op(x, y as u8);
            }
            7 => { // not addr:bit
                y ^= 1 << bit;
                self.write_op(x, y as u8);
            }
            _ => unreachable!()
        }
    }

    fn run(&mut self, target_cycles: i32) -> i32 {
        macro_rules! adjust_op {
            ($op:ident, $x:ident) => ({
                self.cycles(1);
                $x = self.$op($x);
            })
        }

        macro_rules! adjust_addr_op {
            ($op:ident) => ({
                let mut addr = self.read_pc_op() as u16;
                addr |= (self.read_pc_op() as u16) << 8;
                let mut result = self.read_op(addr);
                result = self.$op(result);
                self.write_op(addr, result);
            })
        }

        macro_rules! adjust_dp_op {
            ($op:ident) => ({
                let addr = self.read_pc_op() as u16;
                let mut result = self.read_dp_op(addr);
                result = self.$op(result);
                self.write_dp_op(addr, result);
            })
        }

        macro_rules! adjust_dp_x_op {
            ($op:ident) => ({
                let addr = self.read_pc_op();
                self.cycles(1);
                let mut result = self.read_dp_op(addr + self.reg_x);
                result = self.$op(result);
                self.write_dp_op(addr + self.reg_x, result);
            })
        }

        macro_rules! read_addr_op {
            ($op:ident, $x:ident) => ({
                let mut addr = self.read_pc_op() as u16;
                addr |= (self.read_pc_op() as u16) << 8;
                let y = self.read_op(addr);
                $x = self.$op($x, y);
            })
        }

        macro_rules! read_addr_i_op {
            ($op:ident, $x:ident) => ({
                let mut addr = self.read_pc_op() as u16;
                addr |= (self.read_pc_op() as u16) << 8;
                self.cycles(1);
                let y = self.read_op(addr + $x);
                self.reg_a = self.$op(self.reg_a, y);
            })
        }

        macro_rules! read_const_op {
            ($op:ident, $x:ident) => ({
                let y = self.read_pc_op();
                $x = self.$op($x, y);
            })
        }

        macro_rules! read_dp_op {
            ($op:ident, $x:ident) => ({
                let addr = self.read_pc_op();
                let y = self.read_dp_op(addr);
                $x = self.$op($x, y);
            })
        }

        macro_rules! read_dp_i_op {
            ($op:ident, $x:ident, $y:ident) => ({
                let addr = self.read_pc_op(addr + $y);
                self.cycles(1);
                let z = self.read_dp_op();
                $x = self.$op($x, z);
            })
        }

        macro_rules! read_dpw_op {
            ($op:ident) => ({
                let mut addr = self.read_pc_op();
                let mut x = self.read_dp_op(addr) as u16;
                addr += 1;
                /* TODO:
                if $op != cpw_op {
                    self.cycles(1);
                }
                 */
                x |= (self.read_dp_op(addr) as u16) << 8;
                self.set_reg_ya(self.$op(self.get_reg_ya(), x));
            })
        }

        macro_rules! read_i_dp_x_op {
            ($op:ident) => ({
                let mut addr = self.read_pc_op() + self.reg_x;
                self.cycles(1);
                let addr2 = self.read_dp_op(addr) as u16;
                addr += 1;
                addr2 |= (self.read_dp_op(addr) as u16) << 8;
                let x = self.read_op(addr2);
                self.reg_a = self.$op(reg_a, x);
            })
        }

        macro_rules! read_i_dp_y_op {
            ($op:ident) => ({
                let mut addr = self.read_pc_op();
                self.cycles(1);
                let mut addr2 = self.read_dp_op(addr) as u16;
                addr += 1;
                addr2 |= (self.read_dp_op(addr) as u16) << 8;
                let x = self.read_op(((addr as i16) + ((self.reg_x as i8) as i16)) as u16);
                self.reg_a = self.$op(self.reg_a, x);
            })
        }

        macro_rules! read_i_x_op {
            ($op:ident) => ({
                self.cycles(1);
                let x = self.read_dp_op(self.reg_x);
                self.reg_a = self.$op(self.reg_a, x);
            })
        }

        0 // TODO
    }
}
