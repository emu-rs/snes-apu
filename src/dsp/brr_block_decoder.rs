use super::dsp_helpers;

pub struct BrrBlockDecoder {
    pub is_end: bool,
    pub is_looping: bool,
    samples: [i16; 16],

    sample_index: i32,
    last_sample: i16,
    last_last_sample: i16
}

impl BrrBlockDecoder {
    pub fn new() -> BrrBlockDecoder {
        BrrBlockDecoder {
            is_end: false,
            is_looping: false,
            samples: [0; 16],

            sample_index: 0,

            last_sample: 0,
            last_last_sample: 0
        }
    }

    pub fn reset(&mut self, last_sample: i16, last_last_sample: i16) {
        self.last_sample = last_sample;
        self.last_last_sample = last_last_sample;
    }

    pub fn read(&mut self, buf: &[u8]) {
        let mut buf_pos = 0;

        let raw_header = buf[buf_pos];
        buf_pos += 1;
        self.is_end = (raw_header & 0x01) != 0;
        self.is_looping = (raw_header & 0x02) != 0;

        let filter = (raw_header >> 2) & 0x03;
        let shift = raw_header >> 4;

        let mut out_pos = 0;
        for _ in 0..4 {
            let mut nybbles = buf[buf_pos] as i32;
            buf_pos += 1;
            nybbles = (nybbles << 8) | (buf[buf_pos] as i32);
            buf_pos += 1;

            for _ in 0..4 {
                let mut sample = ((nybbles as i16) >> 12) as i32;
                nybbles <<= 4;

                if shift <= 12 {
                    sample <<= shift;
                    sample >>= 1;
                } else {
                    sample &= !0x07ff;
                }

                let p1 = self.last_sample as i32;
                let p2 = (self.last_last_sample >> 1) as i32;

                match filter {
                    1 => {
                        // sample += p1 * 0.46875
                        sample += p1 >> 1;
                        sample += (-p1) >> 5;
                    },
                    2 => {
                        // sample += p1 * 0.953125 - p2 * 0.46875
                        sample += p1;
                        sample -= p2;
                        sample += p2 >> 4;
                        sample += (p1 * -3) >> 6;
                    },
                    3 => {
                        // sample += p1 * 0.8984375 - p2 * 0.40625
                        sample += p1;
                        sample -= p2;
                        sample += (p1 * -13) >> 7;
                        sample += (p2 * 3) >> 4;
                    },
                    _ => ()
                }

                sample = dsp_helpers::clamp(sample);
                let sample_16 = (sample << 1) as i16;
                self.samples[out_pos] = sample_16;
                out_pos += 1;
                self.last_last_sample = self.last_sample;
                self.last_sample = sample_16;
            }
        }

        self.sample_index = 0;
    }

    pub fn read_next_sample(&mut self) -> i16 {
        let ret = self.samples[self.sample_index as usize];
        self.sample_index += 1;
        ret
    }

    pub fn is_finished(&self) -> bool {
        self.sample_index >= 16
    }
}
