use common::{DbResult, error::DbError};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Constant {
    Integer(i32),
    Varchar(String),
}

impl Constant {
    pub fn as_i32(&self) -> DbResult<i32> {
        match self {
            Self::Integer(value) => Ok(*value),
            _ => Err(DbError::InvalidFieldType),
        }
    }

    pub fn as_str(&self) -> DbResult<&str> {
        match self {
            Self::Varchar(value) => Ok(value),
            _ => Err(DbError::InvalidFieldType),
        }
    }
}

impl std::fmt::Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(value) => write!(f, "{}", value),
            Self::Varchar(value) => write!(f, "'{}'", value),
        }
    }
}
