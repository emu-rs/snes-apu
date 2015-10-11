const NUM_TAPS: usize = 8;

pub struct Filter {
    pub coefficients: [u8; NUM_TAPS],

    buffer: [i32; NUM_TAPS],
    buffer_pos: i32
}

impl Filter {
    pub fn new() -> Filter {
        Filter {
            coefficients: [0; NUM_TAPS],

            buffer: [0; NUM_TAPS],
            buffer_pos: 0
        }
    }

    pub fn next(&mut self, value: i32) -> i32 {
        self.buffer[self.buffer_pos as usize] = value;

        let mut ret = 0;
        for i in 0..NUM_TAPS {
            ret += (self.buffer[((self.buffer_pos + (i as i32)) as usize) % NUM_TAPS] * ((self.coefficients[i] as i8) as i32)) >> 7;
        }

        self.buffer_pos = match self.buffer_pos {
            0 => (NUM_TAPS as i32) - 1,
            _ => self.buffer_pos - 1
        };

        ret
    }
}
