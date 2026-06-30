use common::DbResult;
use common::error::DbError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Element {
    Raw(String),
    View(String, String),
    Spec(String, String),
}

impl Element {
    pub fn raw(value: &str) -> Self {
        Self::Raw(value.to_string())
    }

    pub fn view(source: &str, name: &str) -> Self {
        Self::View(source.to_string(), name.to_string())
    }

    pub fn spec(source: &str, target: &str) -> Self {
        Self::Spec(source.to_string(), target.to_string())
    }

    pub fn as_raw(&self) -> DbResult<&str> {
        match self {
            Self::Raw(s) => Ok(s),
            _ => Err(DbError::InvalidFieldType),
        }
    }
}

impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(value) => write!(f, "{}", value),
            Self::View(source, name) => write!(f, "{} {}", source, name),
            Self::Spec(source, target) => write!(f, "{}.{}", source, target),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view() {
        let view = Element::view("table", "t");
        assert_eq!("table t", view.to_string());
    }

    #[test]
    fn spec() {
        let spec = Element::spec("t1", "f1");
        assert_eq!("t1.f1", spec.to_string());
    }
}
