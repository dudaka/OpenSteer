//! Little-endian reads over a file's bytes, as sro-polyanya's `parse<T>` over an istringstream.

use crate::Error;

pub struct Reader<'a> {
    name: &'a str,
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(name: &'a str, data: &'a [u8]) -> Self {
        Reader { name, data, pos: 0 }
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos + n;
        let out = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| self.error("truncated"))?;
        self.pos = end;
        Ok(out)
    }

    pub fn skip(&mut self, n: usize) -> Result<(), Error> {
        self.bytes(n).map(|_| ())
    }

    /// `seekg(offset, beg)`.
    pub fn seek(&mut self, offset: usize) -> Result<(), Error> {
        if offset > self.data.len() {
            return Err(self.error("seek past the end"));
        }
        self.pos = offset;
        Ok(())
    }

    pub fn error(&self, what: &str) -> Error {
        Error::Format {
            path: self.name.to_string(),
            what: format!("{what} at byte {}", self.pos),
        }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        Ok(self.bytes(N)?.try_into().expect("N bytes"))
    }

    pub fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.array::<1>()?[0])
    }

    pub fn i8(&mut self) -> Result<i8, Error> {
        Ok(self.u8()? as i8)
    }

    pub fn u16(&mut self) -> Result<u16, Error> {
        self.array().map(u16::from_le_bytes)
    }

    pub fn i16(&mut self) -> Result<i16, Error> {
        self.array().map(i16::from_le_bytes)
    }

    pub fn u32(&mut self) -> Result<u32, Error> {
        self.array().map(u32::from_le_bytes)
    }

    pub fn i32(&mut self) -> Result<i32, Error> {
        self.array().map(i32::from_le_bytes)
    }

    pub fn f32(&mut self) -> Result<f32, Error> {
        self.array().map(f32::from_le_bytes)
    }

    /// A u32 length, then that many bytes as text.
    pub fn string(&mut self) -> Result<String, Error> {
        let n = self.u32()? as usize;
        Ok(String::from_utf8_lossy(self.bytes(n)?).into_owned())
    }
}
