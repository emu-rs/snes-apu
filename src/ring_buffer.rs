// TODO: This should really be a generic parameter on RingBuffer,
// but rust does not currently have any facilities for this.
use super::apu::BUFFER_LEN;

pub struct RingBuffer {
    left_buffer: Box<[i16; BUFFER_LEN]>,
    right_buffer: Box<[i16; BUFFER_LEN]>,
    write_pos: i32,
    read_pos: i32,
    sample_count: i32
}

impl RingBuffer {
    pub fn new() -> RingBuffer {
        RingBuffer {
            left_buffer: Box::new([0; BUFFER_LEN]),
            right_buffer: Box::new([0; BUFFER_LEN]),
            write_pos: 0,
            read_pos: 0,
            sample_count: 0
        }
    }

    pub fn set_write_pos(&mut self, pos: i32) {
        self.write_pos = pos % (BUFFER_LEN as i32);
    }

    pub fn set_read_pos(&mut self, pos: i32) {
        self.read_pos = pos % (BUFFER_LEN as i32);
    }

    pub fn write(&mut self, left: &[i16], right: &[i16], num_samples: i32) {
        for i in 0..num_samples {
            self.left_buffer[self.write_pos as usize] = left[i as usize];
            self.right_buffer[self.write_pos as usize] = right[i as usize];
            self.write_pos = (self.write_pos + 1) % (BUFFER_LEN as i32);
        }
        self.sample_count += num_samples;
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
