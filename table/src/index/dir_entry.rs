use crate::constant::Constant;

pub struct DirEntry {
    pub value: Constant,
    pub block_num: i32,
}

impl DirEntry {
    pub fn new(value: Constant, block_num: i32) -> Self {
        Self { value, block_num }
    }
}
