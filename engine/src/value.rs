use common::{DbResult, error::DbError};
use file::page::{I32_SIZE, Page};

pub const VARCHAR_TYPE: u8 = 1;
pub const INTEGER_TYPE: u8 = 2;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub enum Value {
    Integer(i32),
    Varchar(String),
}

impl Value {
    pub fn varchar(value: &str) -> Self {
        Self::Varchar(value.to_string())
    }

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

    pub fn as_string(&self) -> DbResult<String> {
        Ok(self.as_str()?.to_string())
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Integer(_) => I32_SIZE,
            Self::Varchar(value) => Page::str_space(value),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(value) => write!(f, "{}", value),
            Self::Varchar(value) => write!(f, "'{}'", value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_value() {
        let value = Value::varchar("test");
        assert_eq!(value.as_string().unwrap(), "test");
        assert!(matches!(value.as_i32(), Err(DbError::InvalidFieldType)));
    }

    #[test]
    fn int_value() {
        let value = Value::Integer(10);
        assert_eq!(value.as_i32().unwrap(), 10);
        assert!(matches!(value.as_str(), Err(DbError::InvalidFieldType)));
    }

    #[test]
    fn to_string() {
        let value = Value::Integer(10);
        assert_eq!(value.to_string(), "10");

        let value = Value::varchar("1");
        assert_eq!(value.to_string(), "'1'");
    }
}
