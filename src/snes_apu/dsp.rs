// TODO: This is a placeholder before I start generalizing traits
// from the old code.
use super::apu::Apu;
use super::voice::Voice;
use super::filter::Filter;

const NUM_VOICES: usize = 8;

const COUNTER_RANGE: i32 = 30720;
static COUNTER_RATES: [i32; 32] = [
    COUNTER_RANGE + 1, // Never fires
    2048, 1536, 1280, 1024, 768, 640, 512, 384, 320, 256, 192, 160, 128, 96,
    80, 64, 48, 40, 32, 24, 20, 16, 12, 10, 8, 6, 5, 4, 3, 2, 1];

static COUNTER_OFFSETS: [i32; 32] = [
    1, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040,
    536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 0, 0];

pub struct Dsp {
    emulator: *mut Apu,

    voices: Vec<Box<Voice>>,

    left_filter: Box<Filter>,
    right_filter: Box<Filter>,

    /*left_output_buffer: i16,
    right_output_buffer: i16,
    output_index: i32,*/

    vol_left: u8,
    vol_right: u8,
    echo_vol_left: u8,
    echo_vol_right: u8,
    noise_clock: u8,
    echo_write_enabled: bool,
    echo_feedback: u8,
    source_dir: u8,
    echo_start_address: u16,
    echo_delay: u8,

    counter: i32,

    cycles_since_last_flush: i32,
    is_flushing: bool,
    noise: i32,
    echo_pos: i32,
    echo_length: i32
}

impl Dsp {
    pub fn new(emulator: *mut Apu) -> Box<Dsp> {
        let mut ret = Box::new(Dsp {
            emulator: emulator,

            voices: Vec::with_capacity(NUM_VOICES),

            left_filter: Box::new(Filter::new()),
            right_filter: Box::new(Filter::new()),

            // TODO

            vol_left: 0,
            vol_right: 0,
            echo_vol_left: 0,
            echo_vol_right: 0,
            noise_clock: 0,
            echo_write_enabled: false,
            echo_feedback: 0,
            source_dir: 0,
            echo_start_address: 0,
            echo_delay: 0,

            counter: 0,

            cycles_since_last_flush: 0,
            is_flushing: false,
            noise: 0,
            echo_pos: 0,
            echo_length: 0,
        });
        let ret_ptr = &mut *ret as *mut _;
        for i in 0..NUM_VOICES {
            ret.voices.push(Box::new(Voice::new(ret_ptr, emulator)));
        }
        ret.reset();
        ret
    }

    #[inline]
    fn emulator(&self) -> &mut Apu {
        unsafe {
            &mut (*self.emulator)
        }
    }

    fn set_filter_coefficient(&mut self, index: i32, value: u8) {
        self.left_filter.coefficients[index as usize] = value;
        self.right_filter.coefficients[index as usize] = value;
    }

    pub fn reset(&mut self) {
        // TODO: NO idea if some of these are correct
        self.vol_left = 0x89;
        self.vol_right = 0x9c;
        self.echo_vol_left = 0x9f;
        self.echo_vol_right = 0x9c;
        self.noise_clock = 0;
        self.echo_write_enabled = false;
        self.echo_feedback = 0;
        self.source_dir = 0;
        self.echo_start_address = 0x60 << 8; // TODO: This shift gets repeated; abstract?
        self.echo_delay = 0x0e;

        self.set_filter_coefficient(0x00, 0x80);
        self.set_filter_coefficient(0x01, 0xff);
        self.set_filter_coefficient(0x02, 0x9a);
        self.set_filter_coefficient(0x03, 0xff);
        self.set_filter_coefficient(0x04, 0x67);
        self.set_filter_coefficient(0x05, 0xff);
        self.set_filter_coefficient(0x06, 0x0f);
        self.set_filter_coefficient(0x07, 0xff);

        self.counter = 0;

        self.cycles_since_last_flush = 0;
        self.is_flushing = false;
        self.noise = 0x4000;
        self.echo_pos = 0;
        self.echo_length = 0;
    }

    pub fn cycles_callback(&mut self, num_cycles: i32) {
        self.cycles_since_last_flush = self.cycles_since_last_flush + num_cycles;
    }

    pub fn flush(&mut self) {
        while self.cycles_since_last_flush > 64 {
            if !self.read_counter(self.noise_clock as i32) {
                let feedback = (self.noise << 13) ^ (self.noise << 14);
                self.noise = (feedback & 0x4000) ^ (self.noise >> 1);
            }

            let mut left_out = 0;
            let mut right_out = 0;
            let mut left_echo_out = 0;
            let mut right_echo_out = 0;
            let mut last_voice_out = 0;
            for j in 0..NUM_VOICES {
                let voice = &mut self.voices[j];


            }

            // TODO

            self.counter = (self.counter + 1) % COUNTER_RANGE;
            self.cycles_since_last_flush -= 64;
        }
    }

    pub fn read_counter(&self, rate: i32) -> bool {
        ((self.counter + COUNTER_OFFSETS[rate as usize]) % COUNTER_RATES[rate as usize]) != 0
    }

    // TODO: Refactor these methods to reduce code duplication
    pub fn read_source_dir_start_address(&self, index: i32) -> u32 {
        let dir_address = (self.source_dir as i32) * 0x100;
        let entry_address = dir_address + index * 4;
        let mut ret = self.emulator().read_u8(entry_address as u32) as u32;
        ret |= (self.emulator().read_u8((entry_address as u32) + 1) as u32) << 8;
        ret
    }

    pub fn read_source_dir_loop_address(&self, index: i32) -> u32 {
        let dir_address = (self.source_dir as i32) * 0x100;
        let entry_address = dir_address + index * 4;
        let mut ret = self.emulator().read_u8(entry_address as u32 + 2) as u32;
        ret |= (self.emulator().read_u8((entry_address as u32) + 3) as u32) << 8;
        ret
    }
}
