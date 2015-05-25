use std::io::Read;

struct BinaryReader<R> {
    read: R
}

impl<R: Read> BinaryReader<R> {
    fn new(read: R) -> BinaryReader<R> {
        BinaryReader { read: read }
    }
    
    fn read_byte(&mut self) -> u8 {
        let mut buf: [u8; 1] = [0; 1];
        self.read.read(&mut buf);
        buf[0]
    }

    fn read_le_u16(&mut self) -> u16 {
        // TODO: Ensure endian
        let mut buf: [u8; 2] = [0; 2];
        self.read.read(&mut buf);
        ((buf[1] as u16) << 8) | (buf[0] as u16)
    }
    
    fn read_le_i32(&mut self) -> i32 {
        // TODO: Ensure endian
        let mut buf: [u8; 4] = [0; 4];
        self.read.read(&mut buf);
        (((buf[3] as u32) << 24) | ((buf[2] as u32) << 16) |
        ((buf[1] as u32) << 8) | (buf[0] as u32)) as i32
    }
}
