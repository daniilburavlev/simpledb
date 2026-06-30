#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RID(i32, i32);

impl RID {
    pub fn new(block_num: i32, slot: i32) -> Self {
        Self(block_num, slot)
    }

    pub fn block_num(&self) -> i32 {
        self.0
    }

    pub fn slot(&self) -> i32 {
        self.1
    }
}

impl std::fmt::Display for RID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.0, self.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_string() {
        let rid = RID::new(0, 0);
        assert_eq!("[0, 0]", rid.to_string());
    }
}
