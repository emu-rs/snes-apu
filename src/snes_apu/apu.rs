use spc::Spc;
use super::timer::Timer;

pub struct Apu {
    ram: [u8; 0x10000],
    timers: [Timer; 3]
}

impl Apu {
    pub fn new() -> Apu {
        Apu {
            // TODO: Randomize ram/rom contents
            ram: [0; 0x10000],

            timers: [Timer::new(256), Timer::new(256), Timer::new(256)]
        }
    }
}
