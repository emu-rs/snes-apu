use super::spc::Spc;
use super::timer::Timer;
use super::ring_buffer::RingBuffer;

pub const RAM_LEN: usize = 0x10000;

pub const IPL_ROM_LEN: usize = 64;
static DEFAULT_IPL_ROM: [u8; IPL_ROM_LEN] = [
    0xcd, 0xef, 0xbd, 0xe8, 0x00, 0xc6, 0x1d, 0xd0,
    0xfc, 0x8f, 0xaa, 0xf4, 0x8f, 0xbb, 0xf5, 0x78,
    0xcc, 0xf4, 0xd0, 0xfb, 0x2f, 0x19, 0xeb, 0xf4,
    0xd0, 0xfc, 0x7e, 0xf4, 0xd0, 0x0b, 0xe4, 0xf5,
    0xcb, 0xf4, 0xd7, 0x00, 0xfc, 0xd0, 0xf3, 0xab,
    0x01, 0x10, 0xef, 0x7e, 0xf4, 0x10, 0xeb, 0xba,
    0xf6, 0xda, 0x00, 0xba, 0xf4, 0xc4, 0xf4, 0xdd,
    0x5d, 0xd0, 0xdb, 0x1f, 0x00, 0x00, 0xc0, 0xff];

const SAMPLE_RATE: usize = 32000;
pub const BUFFER_LEN: usize = SAMPLE_RATE * 2;

pub struct Apu {
    ram: [u8; RAM_LEN],
    ipl_rom: [u8; IPL_ROM_LEN],

    timers: [Timer; 3],

    left_output_buffer: [i16; BUFFER_LEN],
    right_output_buffer: [i16; BUFFER_LEN],
    overflow_buffer: RingBuffer,

    is_ipl_rom_enabled: bool,
    dsp_reg_address: u8
}

impl Apu {
    pub fn new() -> Apu {
        let mut ret = Apu {
            ram: [0; RAM_LEN],
            ipl_rom: [0; IPL_ROM_LEN],

            timers: [Timer::new(256), Timer::new(256), Timer::new(32)],

            left_output_buffer: [0; BUFFER_LEN],
            right_output_buffer: [0; BUFFER_LEN],
            overflow_buffer: RingBuffer::new(),

            is_ipl_rom_enabled: true,
            dsp_reg_address: 0
        };
        ret.reset();
        ret
    }

    pub fn reset(&mut self) {
        // TODO: Randomize ram

        // TODO: Is there a better way to do this?
        for i in 0..IPL_ROM_LEN {
            self.ipl_rom[i] = DEFAULT_IPL_ROM[i];
        }

        for timer in self.timers.iter_mut() {
            timer.reset();
        }

        self.is_ipl_rom_enabled = true;
        self.dsp_reg_address = 0;
    }

    pub fn cpu_cycles_callback(&mut self, num_cycles: i32) {
        // TODO
    }
}
