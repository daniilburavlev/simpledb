use file::page::{I32_SIZE, U16_SIZE};

#[derive(Clone)]
pub enum FieldInfo {
    Integer,
    Varchar(u16),
}

impl FieldInfo {
    pub fn length(&self) -> u16 {
        match self {
            Self::Integer => I32_SIZE as u16,
            Self::Varchar(len) => U16_SIZE as u16 + *len * 4,
        }
    }
}
