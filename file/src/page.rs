use std::hash::Hash;

use bytes::BytesMut;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    buffer: BytesMut,
}

impl Page {
    pub fn new(block_size: usize) -> Self {
        let buffer = vec![0u8; block_size];
        Self::from(buffer.as_slice())
    }

    pub fn set_i32(&mut self, offset: usize, value: i32) {
        let buffer = &mut self.buffer;
        buffer[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    pub fn get_i32(&self, offset: usize) -> i32 {
        let buffer = &self.buffer;
        i32::from_be_bytes(buffer[offset..offset + 4].try_into().unwrap())
    }

    pub fn set_bytes(&mut self, mut offset: usize, bytes: &[u8]) {
        self.set_i32(offset, bytes.len() as i32);
        offset += 4;
        let buffer = &mut self.buffer;
        buffer[offset..offset + bytes.len()].copy_from_slice(bytes);
    }

    pub fn get_bytes(&self, mut offset: usize) -> &[u8] {
        let len = self.get_i32(offset) as usize;
        offset += 4;
        &self.buffer[offset..offset + len]
    }

    pub fn get_string(&self, offset: usize) -> String {
        let bytes = self.get_bytes(offset);
        String::from_utf8_lossy(bytes).to_string()
    }

    pub fn set_string(&mut self, offset: usize, value: String) {
        let bytes = value.as_bytes();
        self.set_bytes(offset, bytes);
    }

    pub fn max_length(value: &str) -> usize {
        4 + value.len()
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
