use crate::value::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BTreePointer {
    pub(crate) value: Value,
    pub(crate) block_num: i32,
}
