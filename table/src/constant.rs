use common::{DbResult, error::DbError};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Constant {
    Integer(i32),
    Varchar(String),
}

impl Constant {
    pub fn varchar(value: &str) -> Self {
        Constant::Varchar(value.to_string())
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
}

impl std::fmt::Display for Constant {
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
    #[should_panic]
    fn invalid_i32() {
        let constant = Constant::Varchar("string".to_string());
        constant.as_i32().unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_str() {
        let constant = Constant::Integer(10);
        constant.as_str().unwrap();
    }

    #[test]
    fn as_str() {
        let value = "test";
        let constant = Constant::varchar(value);
        assert_eq!(value, constant.as_str().unwrap());
    }
}
