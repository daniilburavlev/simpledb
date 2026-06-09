use std::hash::Hash;

use bytes::BytesMut;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    buffer: BytesMut,
}

pub const U8_SIZE: usize = 1;
pub const U16_SIZE: usize = 2;
pub const U32_SIZE: usize = 4;
pub const U64_SIZE: usize = 8;
pub const I32_SIZE: usize = 4;

impl Page {
    pub fn new(block_size: usize) -> Self {
        let buffer = vec![0u8; block_size];
        Self::from(buffer.as_slice())
    }

    pub fn set_u8(&mut self, offset: usize, value: u8) {
        let buffer = &mut self.buffer;
        buffer[offset..offset + U8_SIZE].copy_from_slice(&value.to_be_bytes());
    }

    pub fn get_u8(&self, offset: usize) -> u8 {
        let buffer = &self.buffer;
        u8::from_be_bytes(buffer[offset..offset + U8_SIZE].try_into().unwrap())
    }

    pub fn set_u16(&mut self, offset: usize, value: u16) {
        let buffer = &mut self.buffer;
        buffer[offset..offset + U16_SIZE].copy_from_slice(&value.to_be_bytes());
    }

    pub fn get_u16(&self, offset: usize) -> u16 {
        let buffer = &self.buffer;
        u16::from_be_bytes(buffer[offset..offset + U16_SIZE].try_into().unwrap())
    }

    pub fn set_u32(&mut self, offset: usize, value: u32) {
        let buffer = &mut self.buffer;
        buffer[offset..offset + U32_SIZE].copy_from_slice(&value.to_be_bytes());
    }

    pub fn get_u32(&self, offset: usize) -> u32 {
        let buffer = &self.buffer;
        u32::from_be_bytes(buffer[offset..offset + U32_SIZE].try_into().unwrap())
    }

    pub fn set_u64(&mut self, offset: usize, value: u64) {
        let buffer = &mut self.buffer;
        buffer[offset..offset + U64_SIZE].copy_from_slice(&value.to_be_bytes());
    }

    pub fn get_u64(&self, offset: usize) -> u64 {
        let buffer = &self.buffer;
        u64::from_be_bytes(buffer[offset..offset + U64_SIZE].try_into().unwrap())
    }

    pub fn set_i32(&mut self, offset: usize, value: i32) {
        let buffer = &mut self.buffer;
        buffer[offset..offset + I32_SIZE].copy_from_slice(&value.to_be_bytes());
    }

    pub fn get_i32(&self, offset: usize) -> i32 {
        let buffer = &self.buffer;
        i32::from_be_bytes(buffer[offset..offset + I32_SIZE].try_into().unwrap())
    }

    pub fn set_bytes(&mut self, mut offset: usize, bytes: &[u8]) {
        self.set_u16(offset, bytes.len() as u16);
        offset += U16_SIZE;
        let buffer = &mut self.buffer;
        buffer[offset..offset + bytes.len()].copy_from_slice(bytes);
    }

    pub fn get_bytes(&self, mut offset: usize) -> &[u8] {
        let len = self.get_u16(offset) as usize;
        offset += U16_SIZE;
        &self.buffer[offset..offset + len]
    }

    pub fn get_string(&self, offset: usize) -> String {
        let bytes = self.get_bytes(offset);
        String::from_utf8_lossy(bytes).to_string()
    }

    pub fn set_string(&mut self, offset: usize, value: &str) {
        let bytes = value.as_bytes();
        self.set_bytes(offset, bytes);
    }

    pub fn str_space(value: &str) -> usize {
        U16_SIZE + value.len()
    }

    pub fn bytes_space(len: usize) -> usize {
        U16_SIZE + len
    }

    pub fn contents(&self) -> &[u8] {
        &self.buffer
    }
}

impl From<&[u8]> for Page {
    fn from(bytes: &[u8]) -> Self {
        Self {
            buffer: BytesMut::from(bytes),
        }
    }
}

impl Hash for Page {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.buffer.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use std::hash::{DefaultHasher, Hasher};

    use super::*;

    #[test]
    fn hash() {
        let p1 = Page::new(10);
        let p2 = Page::new(10);
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        p1.hash(&mut h1);
        p2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }
}
