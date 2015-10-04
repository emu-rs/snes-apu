// TODO: This is a placeholder before I start generalizing traits
// from the old code.
use super::apu::Apu;

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

    counter: i32,
    cycles_since_last_flush: i32,
    noise: i32
}

impl Dsp {
    pub fn new(emulator: *mut Apu) -> Dsp {
        let mut ret = Dsp {
            emulator: emulator,

            counter: 0,
            cycles_since_last_flush: 0,
            noise: 0
        };
        ret.reset();
        ret
    }

    pub fn reset(&mut self) {
        // TODO: Proper impl
        self.counter = 0;
        self.cycles_since_last_flush = 0;
        self.noise = 0x4000;
    }

    pub fn cycles_callback(&mut self, num_cycles: i32) {
        self.cycles_since_last_flush = self.cycles_since_last_flush + num_cycles;
    }

    fn read_counter(&self, rate: i32) -> bool {
        ((self.counter + COUNTER_OFFSETS[rate as usize]) % COUNTER_RATES[rate as usize]) != 0
    }

    pub fn flush(&mut self) {
        while self.cycles_since_last_flush > 64 {
            if !self.read_counter(self.noise_clock) {
                let feedback = (self.noise << 13) ^ (self.noise << 14);
                self.noise = (feedback & 0x4000) ^ (self.noise >> 1);
            }

            let mut left_out = 0;
            let mut right_out = 0;
            let mut left_echo_out = 0;
            let mut right_echo_out = 0;
            let mut last_voice_out = 0;
            for j in 0..NUM_VOICES {
            }
        }
    }
}
