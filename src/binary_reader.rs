use std::io::{Result, Error, Read, Seek, SeekFrom, ErrorKind};

pub trait ReadAll : Read {
    fn read_all(&mut self, buf: &mut [u8]) -> Result<()>;
}

pub trait BinaryRead : ReadAll {
    fn read_u8(&mut self) -> Result<u8>;
    fn read_le_u16(&mut self) -> Result<u16>;
    fn read_le_i32(&mut self) -> Result<i32>;
}

pub struct BinaryReader<R> {
    inner: R
}

impl<R: Read> BinaryReader<R> {
    pub fn new(inner: R) -> BinaryReader<R> {
        BinaryReader { inner: inner }
    }
}

impl<R: Read> Read for BinaryReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: Seek> Seek for BinaryReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.inner.seek(pos)
    }
}

impl<R: Read> ReadAll for BinaryReader<R> {
    fn read_all(&mut self, buf: &mut [u8]) -> Result<()> {
        match self.inner.read(buf) {
            Ok(len) if len == buf.len() => Ok(()),
            Ok(_) => Err(Error::new(ErrorKind::Other, "Could not read all bytes")),
            Err(e) => Err(e)
        }
    }
}

impl<R: Read> BinaryRead for BinaryReader<R> {
    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0; 1];
        try!(self.read_all(&mut buf));
        Ok(buf[0])
    }

    fn read_le_u16(&mut self) -> Result<u16> {
        // TODO: Ensure endian
        let mut buf = [0; 2];
        try!(self.read_all(&mut buf));
        Ok(((buf[1] as u16) << 8) | (buf[0] as u16))
    }

    fn read_le_i32(&mut self) -> Result<i32> {
        // TODO: Ensure endian
        let mut buf = [0; 4];
        try!(self.read_all(&mut buf));
        Ok((
            ((buf[3] as u32) << 24) | ((buf[2] as u32) << 16) |
            ((buf[1] as u32) << 8) | (buf[0] as u32)) as i32)
    }
}
