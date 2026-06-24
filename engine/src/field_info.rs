use common::{DbResult, error::DbError};
use file::page::I32_SIZE;

const INTEGER_TYPE: i32 = 1;
const VARCHAR_TYPE: i32 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldInfo {
    Integer,
    Varchar(i32),
}

impl FieldInfo {
    pub fn new(field_type: i32, len: i32) -> DbResult<Self> {
        let info = match field_type {
            INTEGER_TYPE => Self::Integer,
            VARCHAR_TYPE => Self::Varchar((len - I32_SIZE as i32) / 4),
            _ => return Err(DbError::UnknownType),
        };
        Ok(info)
    }

    pub fn type_id(&self) -> i32 {
        match self {
            Self::Integer => INTEGER_TYPE,
            Self::Varchar(_) => VARCHAR_TYPE,
        }
    }
    pub fn length(&self) -> i32 {
        match self {
            Self::Integer => I32_SIZE as i32,
            Self::Varchar(len) => I32_SIZE as i32 + *len * 4,
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
