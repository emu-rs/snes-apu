struct RingBuffer {
    size: i32,
    // TODO: left_buffer, right_buffer
    write_pos: i32,
    read_pos: i32,
    sample_count: i32
}

/*impl RingBuffer {
    pub fn new() -> RingBuffer {
    }
}*/
