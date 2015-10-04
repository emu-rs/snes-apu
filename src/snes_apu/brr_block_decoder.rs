use super::dsp_helpers::clamp;

struct BrrBlockDecoder {
    is_end: bool,
    is_looping: bool,
    samples: [i16; 16],

    sample_index: i32,
    last_sample: i16,
    last_last_sample: i16
}

impl BrrBlockDecoder {
    pub fn new() -> BrrBlockDecoder {
        let mut ret = BrrBlockDecoder {
            is_end: false,
            is_looping: false,
            samples: [0; 16],

            sample_index: 0,

            last_sample: 0,
            last_last_sample: 0
        };
        ret.reset(0, 0);
        ret
    }

    pub fn reset(&mut self, last_sample: i16, last_last_sample: i16) {
        self.last_sample = last_sample;
        self.last_last_sample = last_last_sample;
    }

    pub fn read(&mut self, buf: &[u8]) {
        let mut buf_pos = 0;

        let raw_header = buf[buf_pos];
        buf_pos = buf_pos + 1;
        self.is_end = (raw_header & 0x01) != 0;
        self.is_looping = (raw_header & 0x02) != 0;

        let filter = (raw_header >> 2) & 0x03;
        let shift = raw_header >> 4;

        let out_pos = 0;
        for i in 0..4 {
            let nybbles = buf[buf_pos];
            buf_pos = buf_pos + 1;
            nybbles = (nybbles << 8) | buf[buf_pos];
            buf_pos = buf_pos + 1;

            for j in 0..4 {
                let sample = ((nybbles >> 12) as i16) as i32;
                nybbles = nybbles << 4;

                if shift <= 12 {
                    sample = sample << shift;
                    sample = sample >> 1;
                } else {
                    sample = sample & !0x07ff;
                }

                let p1 = self.last_sample as i32;
                let p2 = (self.last_last_sample >> 1) as i32;

                match filter {
                    1 => {
                        // sample += p1 * 0.46875
                        sample = sample + (p1 >> 1);
                        sample = sample + ((-p1) >> 5);
                    },
                    2 => {
                        // sample += p1 * 0.953125 - p2 * 0.46875
                        sample = sample + p1;
                        sample = sample - p2;
                        sample = sample + (p2 >> 4);
                        sample = sample + ((p1 * -3) >> 6);
                    },
                    3 => {
                        // sample += p1 * 0.8984375 - p2 * 0.40625
                        sample = sample + p1;
                        sample = sample - p2;
                        sample = sample + ((p1 * -13) >> 7);
                        sample = sample + ((p2 * 3) >> 4);
                    },
                    _ => ()
                }

                sample = clamp(sample);
            }
        }

        self.sample_index = 0;
    }
}
