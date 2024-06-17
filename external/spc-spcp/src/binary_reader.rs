use std::io::{Result, Read, Seek, SeekFrom, ErrorKind};
use super::string_decoder::decode_string;

pub trait BinaryRead : Read {
    fn read_u8(&mut self) -> Result<u8>;
    fn read_le_u16(&mut self) -> Result<u16>;
    fn read_le_u32(&mut self) -> Result<u32>;
    fn read_string(&mut self, len: i32) -> Result<String>;
    fn read_variadic_string(&mut self, max_len: i32) -> Result<String>;
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

impl<R: Read> BinaryRead for BinaryReader<R> {
    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_le_u16(&mut self) -> Result<u16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_le_u32(&mut self) -> Result<u32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_string(&mut self, max_len: i32) -> Result<String> {
        let string_bytes = (0..max_len)
            .map(|_| self.read_u8())
            .collect::<Result<Vec<u8>>>()?;

        let end = string_bytes
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(string_bytes.len());

        decode_string(&string_bytes[..end])
    }

    fn read_variadic_string(&mut self, max_len: i32) -> Result<String> {
        let string_bytes = (0..max_len)
            .map(|_| self.read_u8())
            .take_while(|r|  match r {
                Ok(0) => false,
                _ => true
            })
            .collect::<Result<Vec<u8>>>()?;

        decode_string(&string_bytes)
    }
}
