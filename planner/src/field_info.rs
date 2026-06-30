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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_id() {
        let info = FieldInfo::new(1, 16).unwrap();
        assert_eq!(info.type_id(), INTEGER_TYPE);

        let info = FieldInfo::new(2, 0).unwrap();
        assert_eq!(info.type_id(), VARCHAR_TYPE);
    }

    #[test]
    #[should_panic]
    fn invalid_type() {
        FieldInfo::new(123, 32).unwrap();
    }

    #[test]
    fn to_string() {
        let info = FieldInfo::new(1, 0).unwrap();
        assert_eq!("INTEGER", info.to_string());

        let info = FieldInfo::new(2, 12).unwrap();
        assert_eq!("VARCHAR(2)", info.to_string());
    }
}
