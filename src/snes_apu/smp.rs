// TODO: This will panic on certain opcodes. The ideas I have for cpu_lang
// will probably fix it; we'll see. For now tho, I'm gonna leave it as-is;
// I can imagine fixing it trivially might lead to performance problems.
// Another issue I foresee is many of the opcodes assume the host architecture
// is little-endian, which won't always be the case. I'm not yet sure how
// to handle that yet.
// Finally there's a lot of duplication, particularly when opcodes are
// reading/writing 16-bit values. This can be greatly improved.
// I'll deal with these problems after the port is finished.

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
        self.reg_pc += ((offset as i8) as i16) as u16;
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
        self.reg_pc += ((y as i8) as i16) as u16;
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

    fn set_bit_op(&mut self, opcode: u8) {
        let addr = self.read_pc_op();
        let x = self.read_dp_op(addr) & !(1 << (opcode >> 5));
        self.write_dp_op(addr, x | ((if opcode & 0x10 == 0 { 1 } else { 0 }) << (opcode >> 5)));
    }

    fn test_addr_op(&mut self, x: bool) {
        let mut addr = self.read_pc_op() as u16;
        addr |= (self.read_pc_op() as u16) << 8;
        let y = self.read_op(addr);
        let mut reg_a = self.reg_a;
        self.set_psw_n_z_op((reg_a - y) as u32);
        self.read_op(addr);
        reg_a = self.reg_a;
        self.write_op(addr, if x { y | reg_a } else { y & !reg_a });
    }

    fn write_addr_op(&mut self, x: u8) {
        let mut addr = self.read_pc_op() as u16;
        addr |= (self.read_pc_op() as u16) << 8;
        self.read_op(addr);
        self.write_op(addr, x);
    }

    fn write_addr_i_op(&mut self, x: u8) {
        let mut addr = self.read_pc_op() as u16;
        addr |= (self.read_pc_op() as u16) << 8;
        self.cycles(1);
        addr += x as u16;
        self.read_op(addr);
        let reg_a = self.reg_a;
        self.write_op(addr, reg_a);
    }

    fn write_dp_imm_op(&mut self, x: u8) {
        let addr = self.read_pc_op();
        self.read_dp_op(addr);
        self.write_dp_op(addr, x);
    }

    fn write_dp_i_op(&mut self, x: u8, y: u8) {
        let addr = self.read_pc_op() + y;
        self.cycles(1);
        self.read_dp_op(addr);
        self.write_dp_op(addr, x);
    }

    fn bne_dp_op(&mut self) {
        let addr = self.read_pc_op();
        let x = self.read_dp_op(addr);
        let y = self.read_pc_op();
        self.cycles(1);
        if self.reg_a == x {
            return;
        }
        self.cycles(2);
        // TODO: Some of these casts might not be necessary; there's probably
        // a better way to add a i8 to a u16 with proper signs.
        self.reg_pc += ((y as i8) as i16) as u16;
    }

    fn bne_dp_dec_op(&mut self) {
        let addr = self.read_pc_op();
        let x = self.read_dp_op(addr) - 1;
        self.write_dp_op(addr, x);
        let y = self.read_pc_op();
        if x == 0 {
            return;
        }
        self.cycles(2);
        // TODO: Some of these casts might not be necessary; there's probably
        // a better way to add a i8 to a u16 with proper signs.
        self.reg_pc += ((y as i8) as i16) as u16;
    }

    fn bne_dp_x_op(&mut self) {
        let addr = self.read_pc_op();
        self.cycles(1);
        let reg_x = self.reg_x;
        let x = self.read_dp_op(addr + reg_x);
        let y = self.read_pc_op();
        self.cycles(1);
        if self.reg_a == x {
            return;
        }
        self.cycles(2);
        // TODO: Some of these casts might not be necessary; there's probably
        // a better way to add a i8 to a u16 with proper signs.
        self.reg_pc += ((y as i8) as i16) as u16;
    }

    fn bne_y_dec_op(&mut self) {
        let x = self.read_pc_op();
        self.cycles(2);
        self.reg_y -= 1;
        if self.reg_y == 0 {
            return;
        }
        self.cycles(2);
        // TODO: Some of these casts might not be necessary; there's probably
        // a better way to add a i8 to a u16 with proper signs.
        self.reg_pc += ((x as i8) as i16) as u16;
    }

    fn brk_op(&mut self) {
        let mut addr = self.read_op(0xffde) as u16;
        addr |= (self.read_op(0xffdf) as u16) << 8;
        self.cycles(2);
        let mut reg_pc = self.reg_pc;
        self.write_sp_op((reg_pc >> 8) as u8);
        reg_pc = self.reg_pc;
        self.write_sp_op(reg_pc as u8);
        let psw = self.get_psw();
        self.write_sp_op(psw);
        self.reg_pc = addr;
        self.psw_b = true;
        self.psw_i = false;
    }

    fn clv_op(&mut self) {
        self.cycles(1);
        self.psw_v = false;
        self.psw_h = false;
    }

    fn cmc_op(&mut self) {
        self.cycles(2);
        self.psw_c = !self.psw_c;
    }

    fn daa_op(&mut self) {
        self.cycles(2);
        if self.psw_c || self.reg_a > 0x99 {
            self.reg_a += 0x60;
            self.psw_c = true;
        }
        if self.psw_h || (self.reg_a & 0x0f) > 0x09 {
            self.reg_a += 0x06;
        }
        let reg_a = self.reg_a;
        self.set_psw_n_z_op(reg_a as u32);
    }

    fn das_op(&mut self) {
        self.cycles(2);
        if !self.psw_c || self.reg_a > 0x99 {
            self.reg_a -= 0x60;
            self.psw_c = false;
        }
        if !self.psw_h || (self.reg_a & 0x0f) > 0x09 {
            self.reg_a -= 0x06;
        }
        let reg_a = self.reg_a;
        self.set_psw_n_z_op(reg_a as u32);
    }

    fn div_ya_op(&mut self) {
        self.cycles(11);
        let ya = self.get_reg_ya();
        self.psw_v = self.reg_y >= self.reg_x;
        self.psw_h = (self.reg_y & 0x0f) >= (self.reg_x & 0x0f);
        let reg_x = self.reg_x as u16;
        if (self.reg_y as u16) < (reg_x << 1) {
            self.reg_a = (ya / reg_x) as u8;
            self.reg_y = (ya % reg_x) as u8;
        } else {
            self.reg_a = (255 - (ya - (reg_x << 9)) / (256 - reg_x)) as u8;
            self.reg_y = (reg_x + (ya - (reg_x << 9)) % (256 - reg_x)) as u8;
        }
        let reg_a = self.reg_a;
        self.set_psw_n_z_op(reg_a as u32);
    }

    fn jmp_addr_op(&mut self) {
        let mut addr = self.read_pc_op() as u16;
        addr |= (self.read_pc_op() as u16) << 8;
        self.reg_pc = addr;
    }

    fn jmp_i_addr_x_op(&mut self) {
        let mut addr = self.read_pc_op() as u16;
        addr |= (self.read_pc_op() as u16) << 8;
        self.cycles(1);
        addr += self.reg_x as u16;
        let mut addr2 = self.read_op(addr) as u16;
        addr += 1;
        addr2 |= (self.read_op(addr) as u16) << 8;
        self.reg_pc = addr2;
    }

    fn jsp_dp_op(&mut self) {
        let addr = self.read_pc_op();
        self.cycles(2);
        let mut reg_pc = self.reg_pc;
        self.write_sp_op((reg_pc >> 8) as u8);
        reg_pc = self.reg_pc;
        self.write_sp_op(reg_pc as u8);
        self.reg_pc = 0xff00 | (addr as u16);
    }

    fn jsr_addr_op(&mut self) {
        let mut addr = self.read_pc_op() as u16;
        addr |= (self.read_pc_op() as u16) << 8;
        self.cycles(3);
        let mut reg_pc = self.reg_pc;
        self.write_sp_op((reg_pc >> 8) as u8);
        reg_pc = self.reg_pc;
        self.write_sp_op(reg_pc as u8);
        self.reg_pc = addr;
    }

    fn jst_op(&mut self, opcode: u8) {
        let mut addr = 0xffde - (((opcode as u16) >> 4) << 1);
        let mut addr2 = self.read_op(addr) as u16;
        addr += 1;
        addr2 |= (self.read_op(addr) as u16) << 8;
        self.cycles(3);
        let mut reg_pc = self.reg_pc;
        self.write_sp_op((reg_pc >> 8) as u8);
        reg_pc = self.reg_pc;
        self.write_sp_op(reg_pc as u8);
        self.reg_pc = addr2;
    }

    fn lda_i_x_inc_op(&mut self) {
        self.cycles(1);
        let reg_x = self.reg_x;
        self.reg_x += 1;
        self.reg_a = self.read_dp_op(reg_x);
        self.cycles(1);
        let reg_a = self.reg_a;
        self.set_psw_n_z_op(reg_a as u32);
    }

    fn mul_ya_op(&mut self) {
        self.cycles(8);
        let ya = (self.reg_y as u16) * (self.reg_a as u16);
        self.reg_a = ya as u8;
        self.reg_y = (ya >> 8) as u8;
        let reg_y = self.reg_y;
        self.set_psw_n_z_op(reg_y as u32);
    }

    fn nop_op(&mut self) {
        self.cycles(1);
    }

    fn plp_op(&mut self) {
        self.cycles(2);
        let psw = self.read_sp_op();
        self.set_psw(psw);
    }

    fn rti_op(&mut self) {
        let psw = self.read_sp_op();
        self.set_psw(psw);
        let mut addr = self.read_sp_op() as u16;
        addr |= (self.read_sp_op() as u16) << 8;
        self.cycles(2);
        self.reg_pc = addr;
    }

    fn rts_op(&mut self) {
        let mut addr = self.read_sp_op() as u16;
        addr |= (self.read_sp_op() as u16) << 8;
        self.cycles(2);
        self.reg_pc = addr;
    }

    fn sta_i_dp_x_op(&mut self) {
        let mut addr = self.read_pc_op() + self.reg_x;
        self.cycles(1);
        let mut addr2 = self.read_dp_op(addr) as u16;
        addr += 1;
        addr2 |= (self.read_dp_op(addr) as u16) << 8;
        self.read_op(addr2);
        let reg_a = self.reg_a;
        self.write_op(addr2, reg_a);
    }

    fn sta_i_dp_y_op(&mut self) {
        let mut addr = self.read_pc_op();
        let mut addr2 = self.read_dp_op(addr) as u16;
        addr += 1;
        addr2 |= (self.read_dp_op(addr) as u16) << 8;
        self.cycles(1);
        addr2 += self.reg_y as u16;
        self.read_op(addr2);
        let reg_a = self.reg_a;
        self.write_op(addr2, reg_a);
    }

    fn sta_i_x_op(&mut self) {
        self.cycles(1);
        let mut reg_x = self.reg_x;
        self.read_dp_op(reg_x);
        reg_x = self.reg_x;
        let reg_a = self.reg_a;
        self.write_dp_op(reg_x, reg_a);
    }

    fn sta_i_x_inc_op(&mut self) {
        self.cycles(2);
        let reg_x = self.reg_x;
        self.reg_x += 1;
        let reg_a = self.reg_a;
        self.write_dp_op(reg_x, reg_a);
    }

    fn stw_dp_op(&mut self) {
        let mut addr = self.read_pc_op();
        self.read_dp_op(addr);
        let reg_a = self.reg_a;
        self.write_dp_op(addr, reg_a);
        addr += 1;
        let reg_y = self.reg_y;
        self.write_dp_op(addr, reg_y);
    }

    fn wait_op(&mut self) {
        // TODO
        panic!("wait occurred");
        loop {
            self.cycles(2);
        }
    }

    fn xcn_op(&mut self) {
        self.cycles(4);
        self.reg_a = (self.reg_a >> 4) | (self.reg_a << 4);
        let reg_a = self.reg_a;
        self.set_psw_n_z_op(reg_a as u32);
    }

    fn run(&mut self, target_cycles: i32) -> i32 {
        macro_rules! adjust_op {
            ($op:ident, $x:expr) => ({
                self.cycles(1);
                let temp = $x;
                $x = self.$op(temp);
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
                let addr = self.read_pc_op();
                let mut result = self.read_dp_op(addr);
                result = self.$op(result);
                self.write_dp_op(addr, result);
            })
        }

        macro_rules! adjust_dp_x_op {
            ($op:ident) => ({
                let addr = self.read_pc_op();
                self.cycles(1);
                let mut reg_x = self.reg_x;
                let mut result = self.read_dp_op(addr + reg_x);
                result = self.$op(result);
                reg_x = self.reg_x;
                self.write_dp_op(addr + reg_x, result);
            })
        }

        macro_rules! read_addr_op {
            ($op:ident, $x:expr) => ({
                let mut addr = self.read_pc_op() as u16;
                addr |= (self.read_pc_op() as u16) << 8;
                let y = self.read_op(addr);
                let temp = $x;
                $x = self.$op(temp, y);
            })
        }

        macro_rules! read_addr_i_op {
            ($op:ident, $x:expr) => ({
                let mut addr = self.read_pc_op() as u16;
                addr |= (self.read_pc_op() as u16) << 8;
                self.cycles(1);
                let temp = $x;
                let y = self.read_op(addr + (temp as u16));
                let reg_a = self.reg_a;
                self.reg_a = self.$op(reg_a, y);
            })
        }

        macro_rules! read_const_op {
            ($op:ident, $x:expr) => ({
                let y = self.read_pc_op();
                let temp = $x;
                $x = self.$op(temp, y);
            })
        }

        macro_rules! read_dp_op {
            ($op:ident, $x:expr) => ({
                let addr = self.read_pc_op();
                let y = self.read_dp_op(addr);
                let temp = $x;
                $x = self.$op(temp, y);
            })
        }

        macro_rules! read_dp_i_op {
            ($op:ident, $x:expr, $y:expr) => ({
                let addr = self.read_pc_op();
                self.cycles(1);
                let mut temp = $y;
                let z = self.read_dp_op(addr + temp);
                temp = $x;
                $x = self.$op(temp, z);
            })
        }

        macro_rules! read_dpw_op {
            ($op:ident, $is_cpw_op:expr) => ({
                let mut addr = self.read_pc_op();
                let mut x = self.read_dp_op(addr) as u16;
                addr += 1;
                if is_cpw_op {
                    self.cycles(1);
                }
                x |= (self.read_dp_op(addr) as u16) << 8;
                self.set_reg_ya(self.$op(self.get_reg_ya(), x));
            })
        }

        macro_rules! read_i_dp_x_op {
            ($op:ident) => ({
                let mut addr = self.read_pc_op() + self.reg_x;
                self.cycles(1);
                let mut addr2 = self.read_dp_op(addr) as u16;
                addr += 1;
                addr2 |= (self.read_dp_op(addr) as u16) << 8;
                let x = self.read_op(addr2);
                let reg_a = self.reg_a;
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
                let reg_x = self.reg_x;
                let x = self.read_op(((addr as i16) + ((reg_x as i8) as i16)) as u16);
                let reg_a = self.reg_a;
                self.reg_a = self.$op(reg_a, x);
            })
        }

        macro_rules! read_i_x_op {
            ($op:ident) => ({
                self.cycles(1);
                let reg_x = self.reg_x;
                let x = self.read_dp_op(reg_x);
                let reg_a = self.reg_a;
                self.reg_a = self.$op(reg_a, x);
            })
        }

        macro_rules! set_flag_op {
            ($x:ident, $y:expr, $is_target_psw_i:expr) => ({
                self.cycles(1);
                if $is_target_psw_i {
                    self.cycles(1);
                }
                $x = $y;
            })
        }

        macro_rules! transfer_op {
            ($x:expr, $y:ident, $is_target_reg_sp:expr) => ({
                self.cycles(1);
                $y = $x;
                if !$is_target_reg_sp {
                    self.set_psw_n_z_op($y as u32);
                }
            })
        }

        macro_rules! write_dp_const_op {
            ($op:ident, $is_op_cmp:expr) => ({
                let x = self.read_pc_op();
                let addr = self.read_pc_op();
                let mut y = self.read_dp_op(addr);
                y = self.$op(y, x);
                if !$is_op_cmp {
                    self.write_dp_op(addr, y);
                } else {
                    self.cycles(1);
                }
            })
        }

        macro_rules! write_dp_dp_op {
            ($op:ident, $is_op_cmp:expr, $is_op_st:expr) => ({
                let addr = self.read_pc_op();
                let x = self.read_dp_op(addr);
                let y = self.read_pc_op();
                let mut z = if !$is_op_st { self.read_dp_op(y) } else { 0 };
                z = self.$op(z, x);
                if !$is_op_cmp {
                    self.write_dp_op(y, z);
                } else {
                    self.cycles(1);
                }
            })
        }

        macro_rules! write_i_x_i_y_op {
            ($op:ident, $is_op_cmp:expr) => ({
                self.cycles(1);
                let reg_y = self.reg_y;
                let x = self.read_dp_op(reg_y);
                let reg_x = self.reg_x;
                let mut y = self.read_dp_op(reg_x);
                y = self.$op(y, x);
                if !$is_op_cmp {
                    let reg_x = self.reg_x;
                    self.write_dp_op(reg_x, y);
                } else {
                    self.cycles(1);
                }
            })
        }

        self.cycle_count = 0;
        while self.cycle_count < target_cycles {
            let opcode = self.read_pc_op();
            match opcode {
                0x00 => self.nop_op(),
                0x01 => self.jst_op(opcode),
                0x02 => self.set_bit_op(opcode),
                0x03 => self.branch_bit_op(opcode),
                0x04 => read_dp_op!(or_op, self.reg_a),
                0x05 => read_addr_op!(or_op, self.reg_a),
                0x06 => read_i_x_op!(or_op),
                0x07 => read_i_dp_x_op!(or_op),
                0x08 => read_const_op!(or_op, self.reg_a),
                0x09 => write_dp_dp_op!(or_op, false, false),
                0x0a => self.set_addr_bit_op(opcode),
                0x0b => adjust_dp_op!(asl_op),
                0x0c => adjust_addr_op!(asl_op),
                0x0d => { let psw = self.get_psw(); self.push_op(psw); },
                0x0e => self.test_addr_op(true),
                0x0f => self.brk_op(),
                0x10 => { let psw_n = self.psw_n; self.branch_op(!psw_n); },
                0x11 => self.jst_op(opcode),
                0x12 => self.set_bit_op(opcode),
                0x13 => self.branch_bit_op(opcode),
                0x14 => read_dp_i_op!(or_op, self.reg_a, self.reg_x),
                0x15 => read_addr_i_op!(or_op, self.reg_x),
                0x16 => read_addr_i_op!(or_op, self.reg_y),
                0x17 => read_i_dp_y_op!(or_op),
                0x18 => write_dp_const_op!(or_op, false),
                0x19 => write_i_x_i_y_op!(or_op, false),
                0x1a => self.adjust_dpw_op(!0),
                0x1b => adjust_dp_x_op!(asl_op),
                0x1c => adjust_op!(asl_op, self.reg_a),
                0x1d => adjust_op!(dec_op, self.reg_x),
                0x1e => read_addr_op!(cmp_op, self.reg_x),
                0x1f => self.jmp_i_addr_x_op(),
                // TODO
                _ => panic!("Invalid opcode")
            }
        }

        self.cycle_count
    }
}
