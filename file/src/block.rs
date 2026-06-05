use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockId {
    pub filename: String,
    pub num: usize,
}

impl BlockId {
    pub fn new(filename: &str, num: usize) -> Self {
        Self {
            filename: filename.to_string(),
            num,
        }
    }
}

impl Hash for BlockId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.filename.hash(state);
        self.num.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use std::hash::{DefaultHasher, Hasher};

    use super::*;

    #[test]
    fn hash() {
        let b1 = BlockId::new("file", 10);
        let b2 = BlockId::new("file", 10);
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        b1.hash(&mut h1);
        b2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }
}
