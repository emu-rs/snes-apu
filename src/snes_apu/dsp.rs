// TODO: This is a placeholder before I start generalizing traits
// from the old code.
use super::apu::Apu;

pub struct Dsp {
    emulator: *mut Apu,

    cycles_since_last_flush: i32,
}

impl Dsp {
    pub fn new(emulator: *mut Apu) -> Dsp {
        let mut ret = Dsp {
            emulator: emulator,

            cycles_since_last_flush: 0
        };
        ret.reset();
        ret
    }

    pub fn reset(&mut self) {
        // TODO: Proper impl
        self.cycles_since_last_flush = 0;
    }

    pub fn cycles_callback(&mut self, num_cycles: i32) {
        self.cycles_since_last_flush = self.cycles_since_last_flush + num_cycles;
    }
}
