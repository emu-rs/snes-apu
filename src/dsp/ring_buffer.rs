use super::dsp::BUFFER_LEN;

pub struct RingBuffer {
    left_buffer: Box<[i16]>,
    right_buffer: Box<[i16]>,
    write_pos: i32,
    read_pos: i32,
    sample_count: i32
}

impl RingBuffer {
    pub fn new() -> RingBuffer {
        RingBuffer {
            left_buffer: vec![0; BUFFER_LEN].into_boxed_slice(),
            right_buffer: vec![0; BUFFER_LEN].into_boxed_slice(),
            write_pos: 0,
            read_pos: 0,
            sample_count: 0
        }
    }

    pub fn write_sample(&mut self, left: i16, right: i16) {
        self.left_buffer[self.write_pos as usize] = left;
        self.right_buffer[self.write_pos as usize] = right;
        self.write_pos = (self.write_pos + 1) % (BUFFER_LEN as i32);
        self.sample_count += 1;
    }

    pub fn read(&mut self, left: &mut [i16], right: &mut [i16], num_samples: i32) {
        for i in 0..num_samples {
            left[i as usize] = self.left_buffer[self.read_pos as usize];
            right[i as usize] = self.right_buffer[self.read_pos as usize];
            self.read_pos = (self.read_pos + 1) % (BUFFER_LEN as i32);
        }
        self.sample_count -= num_samples;
    }

    pub fn get_sample_count(&self) -> i32 {
        self.sample_count
    }
}
