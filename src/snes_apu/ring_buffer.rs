use super::apu::SAMPLE_RATE;

// TODO: This should really be a generic parameter on RingBuffer,
// but rust does not currently have any facilities for this.
const BUFFER_SIZE: usize = SAMPLE_RATE * 2;

struct RingBuffer {
    left_buffer: [i16; BUFFER_SIZE],
    right_buffer: [i16; BUFFER_SIZE],
    write_pos: i32,
    read_pos: i32,
    sample_count: i32
}

impl RingBuffer {
    pub fn new() -> RingBuffer {
        RingBuffer {
            left_buffer: [0; BUFFER_SIZE],
            right_buffer: [0; BUFFER_SIZE],
            write_pos: 0,
            read_pos: 0,
            sample_count: 0
        }
    }
}
