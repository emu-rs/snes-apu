use super::apu::Apu;

pub struct Smp {
    emulator: *mut Apu,

    pub reg_pc: u16,
    pub reg_a: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub reg_sp: u8,

    psw_c: bool,
    psw_z: bool,
    psw_h: bool,
    psw_p: bool,
    psw_v: bool,
    psw_n: bool,
    psw_i: bool,
    psw_b: bool,

    is_stopped: bool,

    cycle_count: i32
}

impl Smp {
    pub fn new(emulator: *mut Apu) -> Smp {
        Smp {
            emulator: emulator,

            reg_pc: 0xffc0,
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_sp: 0xef,

            psw_c: false,
            psw_z: true,
            psw_h: false,
            psw_p: false,
            psw_v: false,
            psw_n: false,
            psw_i: false,
            psw_b: false,

            is_stopped: false,

            cycle_count: 0
        }
    }

    #[inline]
    fn emulator(&self) -> &mut Apu {
        unsafe {
            &mut (*self.emulator)
        }
    }

    pub fn set_reg_ya(&mut self, value: u16) {
        self.reg_a = value as u8;
        self.reg_y = (value >> 8) as u8;
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
        self.emulator().cpu_cycles_callback(num_cycles);
        self.cycle_count += num_cycles;
    }

    fn read(&mut self, addr: u16) -> u8 {
        self.cycles(1);
        self.emulator().read_u8(addr as u32)
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.cycles(1);
        self.emulator().write_u8(addr as u32, value);
    }

    fn read_pc(&mut self) -> u8 {
        let addr = self.reg_pc;
        let ret = self.read(addr);
        self.reg_pc = self.reg_pc.wrapping_add(1);
        ret
    }

    fn read_sp(&mut self) -> u8 {
        self.reg_sp = self.reg_sp.wrapping_add(1);
        let addr = 0x0100 | (self.reg_sp as u16);
        self.read(addr)
    }

    fn write_sp(&mut self, value: u8) {
        let addr = 0x0100 | (self.reg_sp as u16);
        self.reg_sp = self.reg_sp.wrapping_sub(1);
        self.write(addr, value);
    }

    fn read_dp(&mut self, addr: u8) -> u8 {
        let addr = (if self.psw_p { 0x0100 } else { 0 }) | (addr as u16);
        self.read(addr)
    }

    fn write_dp(&mut self, addr: u8, value: u8) {
        let addr = (if self.psw_p { 0x0100 } else { 0 }) | (addr as u16);
        self.write(addr, value);
    }

    fn set_psw_n_z(&mut self, x: u32) {
        self.psw_n = Smp::is_negative(x);
        self.psw_z = x == 0;
    }

    fn adc(&mut self, x: u8, y: u8) -> u8 {
        let x = x as i32;
        let y = y as i32;
        let r = x + y + (if self.psw_c { 1 } else { 0 });
        self.psw_n = Smp::is_negative(r as u32);
        self.psw_v = (!(x ^ y) & (x ^ r) & 0x80) != 0;
        self.psw_h = ((x ^ y ^ r) & 0x10) != 0;
        self.psw_z = (r as u8) == 0;
        self.psw_c = r > 0xff;
        r as u8
    }

    fn and(&mut self, x: u8, y: u8) -> u8 {
        let ret = x & y;
        self.set_psw_n_z(ret as u32);
        ret
    }

    fn asl(&mut self, x: u8) -> u8 {
        self.psw_c = Smp::is_negative(x as u32);
        let ret = x << 1;
        self.set_psw_n_z(ret as u32);
        ret
    }

    fn cmp(&mut self, x: u8, y: u8) -> u8 {
        let r = (x as i32) - (y as i32);
        self.psw_n = (r & 0x80) != 0;
        self.psw_z = (r as u8) == 0;
        self.psw_c = r >= 0;
        x
    }

    fn dec(&mut self, x: u8) -> u8 {
        let ret = x.wrapping_sub(1);
        self.set_psw_n_z(ret as u32);
        ret
    }

    fn eor(&mut self, x: u8, y: u8) -> u8 {
        let ret = x ^ y;
        self.set_psw_n_z(ret as u32);
        ret
    }

    fn inc(&mut self, x: u8) -> u8 {
        let ret = x.wrapping_add(1);
        self.set_psw_n_z(ret as u32);
        ret
    }

    fn ld(&mut self, _: u8, y: u8) -> u8 {
        self.set_psw_n_z(y as u32);
        y
    }

    fn lsr(&mut self, x: u8) -> u8 {
        self.psw_c = (x & 0x01) != 0;
        let ret = x >> 1;
        self.set_psw_n_z(ret as u32);
        ret
    }

    fn or(&mut self, x: u8, y: u8) -> u8 {
        let ret = x | y;
        self.set_psw_n_z(ret as u32);
        ret
    }

    fn rol(&mut self, x: u8) -> u8 {
        let carry = if self.psw_c { 1 } else { 0 };
        self.psw_c = (x & 0x80) != 0;
        let ret = (x << 1) | carry;
        self.set_psw_n_z(ret as u32);
        ret
    }

    fn ror(&mut self, x: u8) -> u8 {
        let carry = if self.psw_c { 0x80 } else { 0 };
        self.psw_c = (x & 0x01) != 0;
        let ret = carry | (x >> 1);
        self.set_psw_n_z(ret as u32);
        ret
    }

    fn sbc(&mut self, x: u8, y: u8) -> u8 {
        self.adc(x, !y)
    }

    fn st(&self, _: u8, y: u8) -> u8 {
        y
    }

    fn adw(&mut self, x: u16, y: u16) -> u16 {
        self.psw_c = false;
        let mut ret = self.adc(x as u8, y as u8) as u16;
        ret |= (self.adc((x >> 8) as u8, (y >> 8) as u8) as u16) << 8;
        self.psw_z = ret == 0;
        ret
    }

    fn cpw(&mut self, x: u16, y: u16) -> u16 {
        let r = (x as i32) - (y as i32);
        self.psw_n = (r & 0x8000) != 0;
        self.psw_z = (r as u16) == 0;
        self.psw_c = r >= 0;
        x
    }

    fn ldw(&mut self, _: u16, y: u16) -> u16 {
        self.psw_n = (y & 0x8000) != 0;
        self.psw_z = y == 0;
        y
    }

    fn sbw(&mut self, x: u16, y: u16) -> u16 {
        self.psw_c = true;
        let mut ret = self.sbc(x as u8, y as u8) as u16;
        ret |= (self.sbc((x >> 8) as u8, (y >> 8) as u8) as u16) << 8;
        self.psw_z = ret == 0;
        ret
    }

    fn adjust_dpw(&mut self, x: u16) {
        let mut addr = self.read_pc();
        let mut result = (self.read_dp(addr) as u16) + x;
        self.write_dp(addr, result as u8);
        addr = addr.wrapping_add(1);
        let mut high = (result >> 8) as u8;
        high = high.wrapping_add(self.read_dp(addr));
        result = ((high as u16) << 8) | (result & 0xff);
        self.write_dp(addr, (result >> 8) as u8);
        self.psw_n = (result & 0x8000) != 0;
        self.psw_z = result == 0;
    }

    fn branch(&mut self, cond: bool) {
        let offset = self.read_pc();
        if !cond {
            return;
        }
        self.cycles(2);
        self.reg_pc = self.reg_pc.wrapping_add(((offset as i8) as i16) as u16);
    }

    fn branch_bit(&mut self, x: u8) {
        let addr = self.read_pc();
        let sp = self.read_dp(addr);
        let y = self.read_pc();
        self.cycles(1);
        if ((sp & (1 << ((x as i32) >> 5))) != 0) == ((x & 0x10) != 0) {
            return;
        }
        self.cycles(2);
        self.reg_pc = self.reg_pc.wrapping_add(((y as i8) as i16) as u16);
    }

    fn push(&mut self, x: u8) {
        self.cycles(2);
        self.write_sp(x);
    }

    fn set_addr_bit(&mut self, opcode: u8) {
        let mut x = self.read_pc() as u16;
        x |= (self.read_pc() as u16) << 8;
        let bit = x >> 13;
        x &= 0x1fff;
        let mut y = self.read(x) as u16;
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
                self.write(x, y as u8);
            }
            7 => { // not addr:bit
                y ^= 1 << bit;
                self.write(x, y as u8);
            }
            _ => unreachable!()
        }
    }

    fn set_bit(&mut self, opcode: u8) {
        let addr = self.read_pc();
        let x = self.read_dp(addr) & !(1 << (opcode >> 5));
        self.write_dp(addr, x | ((if opcode & 0x10 == 0 { 1 } else { 0 }) << (opcode >> 5)));
    }

    fn test_addr(&mut self, x: bool) {
        let mut addr = self.read_pc() as u16;
        addr |= (self.read_pc() as u16) << 8;
        let y = self.read(addr);
        let reg_a = self.reg_a;
        self.set_psw_n_z((reg_a.wrapping_sub(y)) as u32);
        self.read(addr);
        self.write(addr, if x { y | reg_a } else { y & !reg_a });
    }

    fn bne_dp(&mut self) {
        let addr = self.read_pc();
        let x = self.read_dp(addr);
        let y = self.read_pc();
        self.cycles(1);
        if self.reg_a == x {
            return;
        }
        self.cycles(2);
        self.reg_pc = self.reg_pc.wrapping_add(((y as i8) as i16) as u16);
    }

    fn bne_dp_dec(&mut self) {
        let addr = self.read_pc();
        let x = self.read_dp(addr).wrapping_sub(1);
        self.write_dp(addr, x);
        let y = self.read_pc();
        if x == 0 {
            return;
        }
        self.cycles(2);
        self.reg_pc = self.reg_pc.wrapping_add(((y as i8) as i16) as u16);
    }

    fn bne_dp_x(&mut self) {
        let addr = self.read_pc();
        self.cycles(1);
        let reg_x = self.reg_x;
        let x = self.read_dp(addr.wrapping_add(reg_x));
        let y = self.read_pc();
        self.cycles(1);
        if self.reg_a == x {
            return;
        }
        self.cycles(2);
        self.reg_pc = self.reg_pc.wrapping_add(((y as i8) as i16) as u16);
    }

    fn bne_y_dec(&mut self) {
        let x = self.read_pc();
        self.cycles(2);
        self.reg_y = self.reg_y.wrapping_sub(1);
        if self.reg_y == 0 {
            return;
        }
        self.cycles(2);
        self.reg_pc = self.reg_pc.wrapping_add(((x as i8) as i16) as u16);
    }

    fn brk(&mut self) {
        let mut addr = self.read(0xffde) as u16;
        addr |= (self.read(0xffdf) as u16) << 8;
        self.cycles(2);
        let reg_pc = self.reg_pc;
        self.write_sp((reg_pc >> 8) as u8);
        self.write_sp(reg_pc as u8);
        let psw = self.get_psw();
        self.write_sp(psw);
        self.reg_pc = addr;
        self.psw_b = true;
        self.psw_i = false;
    }

    fn clv(&mut self) {
        self.cycles(1);
        self.psw_v = false;
        self.psw_h = false;
    }

    fn cmc(&mut self) {
        self.cycles(2);
        self.psw_c = !self.psw_c;
    }

    fn daa(&mut self) {
        self.cycles(2);
        if self.psw_c || self.reg_a > 0x99 {
            self.reg_a = self.reg_a.wrapping_add(0x60);
            self.psw_c = true;
        }
        if self.psw_h || (self.reg_a & 0x0f) > 0x09 {
            self.reg_a = self.reg_a.wrapping_add(0x06);
        }
        let reg_a = self.reg_a;
        self.set_psw_n_z(reg_a as u32);
    }

    fn das(&mut self) {
        self.cycles(2);
        if !self.psw_c || self.reg_a > 0x99 {
            self.reg_a = self.reg_a.wrapping_sub(0x60);
            self.psw_c = false;
        }
        if !self.psw_h || (self.reg_a & 0x0f) > 0x09 {
            self.reg_a = self.reg_a.wrapping_sub(0x06);
        }
        let reg_a = self.reg_a;
        self.set_psw_n_z(reg_a as u32);
    }

    fn div_ya(&mut self) {
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
        self.set_psw_n_z(reg_a as u32);
    }

    fn jmp_addr(&mut self) {
        let mut addr = self.read_pc() as u16;
        addr |= (self.read_pc() as u16) << 8;
        self.reg_pc = addr;
    }

    fn jmp_i_addr_x(&mut self) {
        let mut addr = self.read_pc() as u16;
        addr |= (self.read_pc() as u16) << 8;
        self.cycles(1);
        addr = addr.wrapping_add(self.reg_x as u16);
        let mut addr2 = self.read(addr) as u16;
        addr = addr.wrapping_add(1);
        addr2 |= (self.read(addr) as u16) << 8;
        self.reg_pc = addr2;
    }

    fn jsp_dp(&mut self) {
        let addr = self.read_pc();
        self.cycles(2);
        let reg_pc = self.reg_pc;
        self.write_sp((reg_pc >> 8) as u8);
        self.write_sp(reg_pc as u8);
        self.reg_pc = 0xff00 | (addr as u16);
    }

    fn jsr_addr(&mut self) {
        let mut addr = self.read_pc() as u16;
        addr |= (self.read_pc() as u16) << 8;
        self.cycles(3);
        let reg_pc = self.reg_pc;
        self.write_sp((reg_pc >> 8) as u8);
        self.write_sp(reg_pc as u8);
        self.reg_pc = addr;
    }

    fn jst(&mut self, opcode: u8) {
        let mut addr = 0xffde - (((opcode >> 4) << 1) as u16);
        let mut addr2 = self.read(addr) as u16;
        addr = addr.wrapping_add(1);
        addr2 |= (self.read(addr) as u16) << 8;
        self.cycles(3);
        let reg_pc = self.reg_pc;
        self.write_sp((reg_pc >> 8) as u8);
        self.write_sp(reg_pc as u8);
        self.reg_pc = addr2;
    }

    fn lda_i_x_inc(&mut self) {
        self.cycles(1);
        let reg_x = self.reg_x;
        self.reg_a = self.read_dp(reg_x);
        self.reg_x = self.reg_x.wrapping_add(1);
        self.cycles(1);
        let reg_a = self.reg_a;
        self.set_psw_n_z(reg_a as u32);
    }

    fn mul_ya(&mut self) {
        self.cycles(8);
        let ya = (self.reg_y as u16) * (self.reg_a as u16);
        self.reg_a = ya as u8;
        self.reg_y = (ya >> 8) as u8;
        let reg_y = self.reg_y;
        self.set_psw_n_z(reg_y as u32);
    }

    fn nop(&mut self) {
        self.cycles(1);
    }

    fn plp(&mut self) {
        self.cycles(2);
        let psw = self.read_sp();
        self.set_psw(psw);
    }

    fn rti(&mut self) {
        let psw = self.read_sp();
        self.set_psw(psw);
        let mut addr = self.read_sp() as u16;
        addr |= (self.read_sp() as u16) << 8;
        self.cycles(2);
        self.reg_pc = addr;
    }

    fn rts(&mut self) {
        let mut addr = self.read_sp() as u16;
        addr |= (self.read_sp() as u16) << 8;
        self.cycles(2);
        self.reg_pc = addr;
    }

    fn sta_i_dp_x(&mut self) {
        let mut addr = self.read_pc() + self.reg_x;
        self.cycles(1);
        let mut addr2 = self.read_dp(addr) as u16;
        addr = addr.wrapping_add(1);
        addr2 |= (self.read_dp(addr) as u16) << 8;
        self.read(addr2);
        let reg_a = self.reg_a;
        self.write(addr2, reg_a);
    }

    fn sta_i_dp_y(&mut self) {
        let mut addr = self.read_pc();
        let mut addr2 = self.read_dp(addr) as u16;
        addr = addr.wrapping_add(1);
        addr2 |= (self.read_dp(addr) as u16) << 8;
        self.cycles(1);
        addr2 = addr2.wrapping_add(self.reg_y as u16);
        self.read(addr2);
        let reg_a = self.reg_a;
        self.write(addr2, reg_a);
    }

    fn sta_i_x(&mut self) {
        self.cycles(1);
        let reg_x = self.reg_x;
        self.read_dp(reg_x);
        let reg_a = self.reg_a;
        self.write_dp(reg_x, reg_a);
    }

    fn sta_i_x_inc(&mut self) {
        self.cycles(2);
        let reg_x = self.reg_x;
        let reg_a = self.reg_a;
        self.write_dp(reg_x, reg_a);
        self.reg_x = self.reg_x.wrapping_add(1);
    }

    fn stw_dp(&mut self) {
        let mut addr = self.read_pc();
        self.read_dp(addr);
        let reg_a = self.reg_a;
        self.write_dp(addr, reg_a);
        addr = addr.wrapping_add(1);
        let reg_y = self.reg_y;
        self.write_dp(addr, reg_y);
    }

    fn sleep_stop(&mut self) {
        self.cycles(2);
        self.is_stopped = true;
    }

    fn xcn(&mut self) {
        self.cycles(4);
        self.reg_a = (self.reg_a >> 4) | (self.reg_a << 4);
        let reg_a = self.reg_a;
        self.set_psw_n_z(reg_a as u32);
    }

    pub fn run(&mut self, target_cycles: i32) -> i32 {
        macro_rules! adjust {
            ($op:ident, $x:expr) => ({
                self.cycles(1);
                let temp = $x;
                $x = self.$op(temp);
            })
        }

        macro_rules! adjust_addr {
            ($op:ident) => ({
                let mut addr = self.read_pc() as u16;
                addr |= (self.read_pc() as u16) << 8;
                let mut result = self.read(addr);
                result = self.$op(result);
                self.write(addr, result);
            })
        }

        macro_rules! adjust_dp {
            ($op:ident) => ({
                let addr = self.read_pc();
                let mut result = self.read_dp(addr);
                result = self.$op(result);
                self.write_dp(addr, result);
            })
        }

        macro_rules! adjust_dp_x {
            ($op:ident) => ({
                let addr = self.read_pc();
                self.cycles(1);
                let mut reg_x = self.reg_x;
                let mut result = self.read_dp(addr.wrapping_add(reg_x));
                result = self.$op(result);
                reg_x = self.reg_x;
                self.write_dp(addr.wrapping_add(reg_x), result);
            })
        }

        macro_rules! read_addr {
            ($op:ident, $x:expr) => ({
                let mut addr = self.read_pc() as u16;
                addr |= (self.read_pc() as u16) << 8;
                let y = self.read(addr);
                let temp = $x;
                $x = self.$op(temp, y);
            })
        }

        macro_rules! read_addr_i {
            ($op:ident, $x:expr) => ({
                let mut addr = self.read_pc() as u16;
                addr |= (self.read_pc() as u16) << 8;
                self.cycles(1);
                let temp = $x;
                let y = self.read(addr.wrapping_add(temp as u16));
                let reg_a = self.reg_a;
                self.reg_a = self.$op(reg_a, y);
            })
        }

        macro_rules! read_const {
            ($op:ident, $x:expr) => ({
                let y = self.read_pc();
                let temp = $x;
                $x = self.$op(temp, y);
            })
        }

        macro_rules! read_dp {
            ($op:ident, $x:expr) => ({
                let addr = self.read_pc();
                let y = self.read_dp(addr);
                let temp = $x;
                $x = self.$op(temp, y);
            })
        }

        macro_rules! read_dp_i {
            ($op:ident, $x:expr, $y:expr) => ({
                let addr = self.read_pc();
                self.cycles(1);
                let mut temp = $y;
                let z = self.read_dp(addr.wrapping_add(temp));
                temp = $x;
                $x = self.$op(temp, z);
            })
        }

        macro_rules! read_dpw {
            ($op:ident, $is_cpw:expr) => ({
                let mut addr = self.read_pc();
                let mut x = self.read_dp(addr) as u16;
                addr = addr.wrapping_add(1);
                if !$is_cpw {
                    self.cycles(1);
                }
                x |= (self.read_dp(addr) as u16) << 8;
                let ya = self.get_reg_ya();
                let ya = self.$op(ya, x);
                self.set_reg_ya(ya);
            })
        }

        macro_rules! read_i_dp_x {
            ($op:ident) => ({
                let mut addr = self.read_pc().wrapping_add(self.reg_x);
                self.cycles(1);
                let mut addr2 = self.read_dp(addr) as u16;
                addr = addr.wrapping_add(1);
                addr2 |= (self.read_dp(addr) as u16) << 8;
                let x = self.read(addr2);
                let reg_a = self.reg_a;
                self.reg_a = self.$op(reg_a, x);
            })
        }

        macro_rules! read_i_dp_y {
            ($op:ident) => ({
                let mut addr = self.read_pc();
                self.cycles(1);
                let mut addr2 = self.read_dp(addr) as u16;
                addr = addr.wrapping_add(1);
                addr2 |= (self.read_dp(addr) as u16) << 8;
                let reg_y = self.reg_y;
                let x = self.read(addr2.wrapping_add(reg_y as u16));
                let reg_a = self.reg_a;
                self.reg_a = self.$op(reg_a, x);
            })
        }

        macro_rules! read_i_x {
            ($op:ident) => ({
                self.cycles(1);
                let reg_x = self.reg_x;
                let x = self.read_dp(reg_x);
                let reg_a = self.reg_a;
                self.reg_a = self.$op(reg_a, x);
            })
        }

        macro_rules! set_flag {
            ($x:expr, $y:expr, $is_dest_psw_i:expr) => ({
                self.cycles(1);
                if $is_dest_psw_i {
                    self.cycles(1);
                }
                $x = $y;
            })
        }

        macro_rules! transfer {
            ($x:expr, $y:expr, $is_dest_reg_sp:expr) => ({
                self.cycles(1);
                $y = $x;
                if !$is_dest_reg_sp {
                    let temp = $y;
                    self.set_psw_n_z(temp as u32);
                }
            })
        }

        macro_rules! write_dp_const {
            ($op:ident, $is_cmp:expr) => ({
                let x = self.read_pc();
                let addr = self.read_pc();
                let mut y = self.read_dp(addr);
                y = self.$op(y, x);
                if !$is_cmp {
                    self.write_dp(addr, y);
                } else {
                    self.cycles(1);
                }
            })
        }

        macro_rules! write_dp_dp {
            ($op:ident, $is_cmp:expr, $is_st:expr) => ({
                let addr = self.read_pc();
                let x = self.read_dp(addr);
                let y = self.read_pc();
                let mut z = if !$is_st { self.read_dp(y) } else { 0 };
                z = self.$op(z, x);
                if !$is_cmp {
                    self.write_dp(y, z);
                } else {
                    self.cycles(1);
                }
            })
        }

        macro_rules! write_i_x_i_y {
            ($op:ident, $is_cmp:expr) => ({
                self.cycles(1);
                let reg_y = self.reg_y;
                let x = self.read_dp(reg_y);
                let reg_x = self.reg_x;
                let mut y = self.read_dp(reg_x);
                y = self.$op(y, x);
                if !$is_cmp {
                    let reg_x = self.reg_x;
                    self.write_dp(reg_x, y);
                } else {
                    self.cycles(1);
                }
            })
        }

        macro_rules! pull {
            ($x:expr) => ({
                self.cycles(2);
                $x = self.read_sp();
            })
        }

        macro_rules! write_dp_imm {
            ($x:expr) => ({
                let addr = self.read_pc();
                self.read_dp(addr);
                let temp = $x;
                self.write_dp(addr, temp);
            })
        }

        macro_rules! write_dp_i {
            ($x:expr, $y:expr) => ({
                let addr = self.read_pc().wrapping_add($y);
                self.cycles(1);
                self.read_dp(addr);
                let temp = $x;
                self.write_dp(addr, temp);
            })
        }

        macro_rules! write_addr {
            ($x:expr) => ({
                let mut addr = self.read_pc() as u16;
                addr |= (self.read_pc() as u16) << 8;
                self.read(addr);
                let temp = $x;
                self.write(addr, temp);
            })
        }

        macro_rules! write_addr_i {
            ($x:expr) => ({
                let mut addr = self.read_pc() as u16;
                addr |= (self.read_pc() as u16) << 8;
                self.cycles(1);
                addr = addr.wrapping_add($x as u16);
                self.read(addr);
                let reg_a = self.reg_a;
                self.write(addr, reg_a);
            })
        }

        self.cycle_count = 0;
        while self.cycle_count < target_cycles {
            if !self.is_stopped {
                let opcode = self.read_pc();
                match opcode {
                    0x00 => self.nop(),
                    0x01 => self.jst(opcode),
                    0x02 => self.set_bit(opcode),
                    0x03 => self.branch_bit(opcode),
                    0x04 => read_dp!(or, self.reg_a),
                    0x05 => read_addr!(or, self.reg_a),
                    0x06 => read_i_x!(or),
                    0x07 => read_i_dp_x!(or),
                    0x08 => read_const!(or, self.reg_a),
                    0x09 => write_dp_dp!(or, false, false),
                    0x0a => self.set_addr_bit(opcode),
                    0x0b => adjust_dp!(asl),
                    0x0c => adjust_addr!(asl),
                    0x0d => { let psw = self.get_psw(); self.push(psw); },
                    0x0e => self.test_addr(true),
                    0x0f => self.brk(),

                    0x10 => { let psw_n = self.psw_n; self.branch(!psw_n); },
                    0x11 => self.jst(opcode),
                    0x12 => self.set_bit(opcode),
                    0x13 => self.branch_bit(opcode),
                    0x14 => read_dp_i!(or, self.reg_a, self.reg_x),
                    0x15 => read_addr_i!(or, self.reg_x),
                    0x16 => read_addr_i!(or, self.reg_y),
                    0x17 => read_i_dp_y!(or),
                    0x18 => write_dp_const!(or, false),
                    0x19 => write_i_x_i_y!(or, false),
                    0x1a => self.adjust_dpw(!0),
                    0x1b => adjust_dp_x!(asl),
                    0x1c => adjust!(asl, self.reg_a),
                    0x1d => adjust!(dec, self.reg_x),
                    0x1e => read_addr!(cmp, self.reg_x),
                    0x1f => self.jmp_i_addr_x(),

                    0x20 => set_flag!(self.psw_p, false, false),
                    0x21 => self.jst(opcode),
                    0x22 => self.set_bit(opcode),
                    0x23 => self.branch_bit(opcode),
                    0x24 => read_dp!(and, self.reg_a),
                    0x25 => read_addr!(and, self.reg_a),
                    0x26 => read_i_x!(and),
                    0x27 => read_i_dp_x!(and),
                    0x28 => read_const!(and, self.reg_a),
                    0x29 => write_dp_dp!(and, false, false),
                    0x2a => self.set_addr_bit(opcode),
                    0x2b => adjust_dp!(rol),
                    0x2c => adjust_addr!(rol),
                    0x2d => { let reg_a = self.reg_a; self.push(reg_a); },
                    0x2e => self.bne_dp(),
                    0x2f => self.branch(true),

                    0x30 => { let psw_n = self.psw_n; self.branch(psw_n); },
                    0x31 => self.jst(opcode),
                    0x32 => self.set_bit(opcode),
                    0x33 => self.branch_bit(opcode),
                    0x34 => read_dp_i!(and, self.reg_a, self.reg_x),
                    0x35 => read_addr_i!(and, self.reg_x),
                    0x36 => read_addr_i!(and, self.reg_y),
                    0x37 => read_i_dp_y!(and),
                    0x38 => write_dp_const!(and, false),
                    0x39 => write_i_x_i_y!(and, false),
                    0x3a => self.adjust_dpw(1),
                    0x3b => adjust_dp_x!(rol),
                    0x3c => adjust!(rol, self.reg_a),
                    0x3d => adjust!(inc, self.reg_x),
                    0x3e => read_dp!(cmp, self.reg_x),
                    0x3f => self.jsr_addr(),

                    0x40 => set_flag!(self.psw_p, true, false),
                    0x41 => self.jst(opcode),
                    0x42 => self.set_bit(opcode),
                    0x43 => self.branch_bit(opcode),
                    0x44 => read_dp!(eor, self.reg_a),
                    0x45 => read_addr!(eor, self.reg_a),
                    0x46 => read_i_x!(eor),
                    0x47 => read_i_dp_x!(eor),
                    0x48 => read_const!(eor, self.reg_a),
                    0x49 => write_dp_dp!(eor, false, false),
                    0x4a => self.set_addr_bit(opcode),
                    0x4b => adjust_dp!(lsr),
                    0x4c => adjust_addr!(lsr),
                    0x4d => { let reg_x = self.reg_x; self.push(reg_x); },
                    0x4e => self.test_addr(false),
                    0x4f => self.jsp_dp(),

                    0x50 => { let psw_v = self.psw_v; self.branch(!psw_v); },
                    0x51 => self.jst(opcode),
                    0x52 => self.set_bit(opcode),
                    0x53 => self.branch_bit(opcode),
                    0x54 => read_dp_i!(eor, self.reg_a, self.reg_x),
                    0x55 => read_addr_i!(eor, self.reg_x),
                    0x56 => read_addr_i!(eor, self.reg_y),
                    0x57 => read_i_dp_y!(eor),
                    0x58 => write_dp_const!(eor, false),
                    0x59 => write_i_x_i_y!(eor, false),
                    0x5a => read_dpw!(cpw, true),
                    0x5b => adjust_dp_x!(lsr),
                    0x5c => adjust!(lsr, self.reg_a),
                    0x5d => transfer!(self.reg_a, self.reg_x, false),
                    0x5e => read_addr!(cmp, self.reg_y),
                    0x5f => self.jmp_addr(),

                    0x60 => set_flag!(self.psw_c, false, false),
                    0x61 => self.jst(opcode),
                    0x62 => self.set_bit(opcode),
                    0x63 => self.branch_bit(opcode),
                    0x64 => read_dp!(cmp, self.reg_a),
                    0x65 => read_addr!(cmp, self.reg_a),
                    0x66 => read_i_x!(cmp),
                    0x67 => read_i_dp_x!(cmp),
                    0x68 => read_const!(cmp, self.reg_a),
                    0x69 => write_dp_dp!(cmp, true, false),
                    0x6a => self.set_addr_bit(opcode),
                    0x6b => adjust_dp!(ror),
                    0x6c => adjust_addr!(ror),
                    0x6d => { let reg_y = self.reg_y; self.push(reg_y); },
                    0x6e => self.bne_dp_dec(),
                    0x6f => self.rts(),

                    0x70 => { let psw_v = self.psw_v; self.branch(psw_v); },
                    0x71 => self.jst(opcode),
                    0x72 => self.set_bit(opcode),
                    0x73 => self.branch_bit(opcode),
                    0x74 => read_dp_i!(cmp, self.reg_a, self.reg_x),
                    0x75 => read_addr_i!(cmp, self.reg_x),
                    0x76 => read_addr_i!(cmp, self.reg_y),
                    0x77 => read_i_dp_y!(cmp),
                    0x78 => write_dp_const!(cmp, true),
                    0x79 => write_i_x_i_y!(cmp, true),
                    0x7a => read_dpw!(adw, false),
                    0x7b => adjust_dp_x!(ror),
                    0x7c => adjust!(ror, self.reg_a),
                    0x7d => transfer!(self.reg_x, self.reg_a, false),
                    0x7e => read_dp!(cmp, self.reg_y),
                    0x7f => self.rti(),

                    0x80 => set_flag!(self.psw_c, true, false),
                    0x81 => self.jst(opcode),
                    0x82 => self.set_bit(opcode),
                    0x83 => self.branch_bit(opcode),
                    0x84 => read_dp!(adc, self.reg_a),
                    0x85 => read_addr!(adc, self.reg_a),
                    0x86 => read_i_x!(adc),
                    0x87 => read_i_dp_x!(adc),
                    0x88 => read_const!(adc, self.reg_a),
                    0x89 => write_dp_dp!(adc, false, false),
                    0x8a => self.set_addr_bit(opcode),
                    0x8b => adjust_dp!(dec),
                    0x8c => adjust_addr!(dec),
                    0x8d => read_const!(ld, self.reg_y),
                    0x8e => self.plp(),
                    0x8f => write_dp_const!(st, false),

                    0x90 => { let psw_c = self.psw_c; self.branch(!psw_c); },
                    0x91 => self.jst(opcode),
                    0x92 => self.set_bit(opcode),
                    0x93 => self.branch_bit(opcode),
                    0x94 => read_dp_i!(adc, self.reg_a, self.reg_x),
                    0x95 => read_addr_i!(adc, self.reg_x),
                    0x96 => read_addr_i!(adc, self.reg_y),
                    0x97 => read_i_dp_y!(adc),
                    0x98 => write_dp_const!(adc, false),
                    0x99 => write_i_x_i_y!(adc, false),
                    0x9a => read_dpw!(sbw, false),
                    0x9b => adjust_dp_x!(dec),
                    0x9c => adjust!(dec, self.reg_a),
                    0x9d => transfer!(self.reg_sp, self.reg_x, false),
                    0x9e => self.div_ya(),
                    0x9f => self.xcn(),

                    0xa0 => set_flag!(self.psw_i, true, true),
                    0xa1 => self.jst(opcode),
                    0xa2 => self.set_bit(opcode),
                    0xa3 => self.branch_bit(opcode),
                    0xa4 => read_dp!(sbc, self.reg_a),
                    0xa5 => read_addr!(sbc, self.reg_a),
                    0xa6 => read_i_x!(sbc),
                    0xa7 => read_i_dp_x!(sbc),
                    0xa8 => read_const!(sbc, self.reg_a),
                    0xa9 => write_dp_dp!(sbc, false, false),
                    0xaa => self.set_addr_bit(opcode),
                    0xab => adjust_dp!(inc),
                    0xac => adjust_addr!(inc),
                    0xad => read_const!(cmp, self.reg_y),
                    0xae => pull!(self.reg_a),
                    0xaf => self.sta_i_x_inc(),

                    0xb0 => { let psw_c = self.psw_c; self.branch(psw_c); },
                    0xb1 => self.jst(opcode),
                    0xb2 => self.set_bit(opcode),
                    0xb3 => self.branch_bit(opcode),
                    0xb4 => read_dp_i!(sbc, self.reg_a, self.reg_x),
                    0xb5 => read_addr_i!(sbc, self.reg_x),
                    0xb6 => read_addr_i!(sbc, self.reg_y),
                    0xb7 => read_i_dp_y!(sbc),
                    0xb8 => write_dp_const!(sbc, false),
                    0xb9 => write_i_x_i_y!(sbc, false),
                    0xba => read_dpw!(ldw, false),
                    0xbb => adjust_dp_x!(inc),
                    0xbc => adjust!(inc, self.reg_a),
                    0xbd => transfer!(self.reg_x, self.reg_sp, true),
                    0xbe => self.das(),
                    0xbf => self.lda_i_x_inc(),

                    0xc0 => set_flag!(self.psw_i, false, true),
                    0xc1 => self.jst(opcode),
                    0xc2 => self.set_bit(opcode),
                    0xc3 => self.branch_bit(opcode),
                    0xc4 => write_dp_imm!(self.reg_a),
                    0xc5 => write_addr!(self.reg_a),
                    0xc6 => self.sta_i_x(),
                    0xc7 => self.sta_i_dp_x(),
                    0xc8 => read_const!(cmp, self.reg_x),
                    0xc9 => write_addr!(self.reg_x),
                    0xca => self.set_addr_bit(opcode),
                    0xcb => write_dp_imm!(self.reg_y),
                    0xcc => write_addr!(self.reg_y),
                    0xcd => read_const!(ld, self.reg_x),
                    0xce => pull!(self.reg_x),
                    0xcf => self.mul_ya(),

                    0xd0 => { let psw_z = self.psw_z; self.branch(!psw_z); },
                    0xd1 => self.jst(opcode),
                    0xd2 => self.set_bit(opcode),
                    0xd3 => self.branch_bit(opcode),
                    0xd4 => write_dp_i!(self.reg_a, self.reg_x),
                    0xd5 => write_addr_i!(self.reg_x),
                    0xd6 => write_addr_i!(self.reg_y),
                    0xd7 => self.sta_i_dp_y(),
                    0xd8 => write_dp_imm!(self.reg_x),
                    0xd9 => write_dp_i!(self.reg_x, self.reg_y),
                    0xda => self.stw_dp(),
                    0xdb => write_dp_i!(self.reg_y, self.reg_x),
                    0xdc => adjust!(dec, self.reg_y),
                    0xdd => transfer!(self.reg_y, self.reg_a, false),
                    0xde => self.bne_dp_x(),
                    0xdf => self.daa(),

                    0xe0 => self.clv(),
                    0xe1 => self.jst(opcode),
                    0xe2 => self.set_bit(opcode),
                    0xe3 => self.branch_bit(opcode),
                    0xe4 => read_dp!(ld, self.reg_a),
                    0xe5 => read_addr!(ld, self.reg_a),
                    0xe6 => read_i_x!(ld),
                    0xe7 => read_i_dp_x!(ld),
                    0xe8 => read_const!(ld, self.reg_a),
                    0xe9 => read_addr!(ld, self.reg_x),
                    0xea => self.set_addr_bit(opcode),
                    0xeb => read_dp!(ld, self.reg_y),
                    0xec => read_addr!(ld, self.reg_y),
                    0xed => self.cmc(),
                    0xee => pull!(self.reg_y),
                    0xef => self.sleep_stop(),

                    0xf0 => { let psw_z = self.psw_z; self.branch(psw_z); },
                    0xf1 => self.jst(opcode),
                    0xf2 => self.set_bit(opcode),
                    0xf3 => self.branch_bit(opcode),
                    0xf4 => read_dp_i!(ld, self.reg_a, self.reg_x),
                    0xf5 => read_addr_i!(ld, self.reg_x),
                    0xf6 => read_addr_i!(ld, self.reg_y),
                    0xf7 => read_i_dp_y!(ld),
                    0xf8 => read_dp!(ld, self.reg_x),
                    0xf9 => read_dp_i!(ld, self.reg_x, self.reg_y),
                    0xfa => write_dp_dp!(st, false, true),
                    0xfb => read_dp_i!(ld, self.reg_y, self.reg_x),
                    0xfc => adjust!(inc, self.reg_y),
                    0xfd => transfer!(self.reg_a, self.reg_y, false),
                    0xfe => self.bne_y_dec(),
                    0xff => self.sleep_stop(),

                    _ => panic!("Invalid opcode")
                }
            } else {
                self.cycles(2);
            }
        }

        self.cycle_count
    }
}
