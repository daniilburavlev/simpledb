use common::{DbResult, error::DbError};
use file::page::{I32_SIZE, U16_SIZE};

const INTEGER_TYPE: u8 = 1;
const VARCHAR_TYPE: u8 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldInfo {
    Integer,
    Varchar(u16),
}

impl FieldInfo {
    pub fn new(field_type: u8, len: u16) -> DbResult<Self> {
        let info = match field_type {
            INTEGER_TYPE => Self::Integer,
            VARCHAR_TYPE => Self::Varchar((len - U16_SIZE as u16) / 4),
            _ => return Err(DbError::UnknownType),
        };
        Ok(info)
    }

    pub fn type_id(&self) -> u8 {
        match self {
            Self::Integer => INTEGER_TYPE,
            Self::Varchar(_) => VARCHAR_TYPE,
        }
    }
    pub fn length(&self) -> u16 {
        match self {
            Self::Integer => I32_SIZE as u16,
            Self::Varchar(len) => U16_SIZE as u16 + *len * 4,
        }
    }
}

impl std::fmt::Display for FieldInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer => write!(f, "INTEGER"),
            Self::Varchar(len) => write!(f, "VARCHAR({})", len),
        }
    }
}
