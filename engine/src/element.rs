use common::DbResult;
use common::error::DbError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Element {
    Raw(String),
    View(String, String),
    Spec(String, String),
    Array(Vec<Box<Self>>),
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

    pub(crate) fn array(values: Vec<Box<Self>>) -> Self {
        Self::Array(values)
    }

    pub fn as_raw(&self) -> DbResult<&str> {
        match self {
            Self::Raw(s) => Ok(s),
            _ => Err(DbError::InvalidFieldType),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Raw(s) => s.len(),
            Self::View(source, name) => source.len() + 4 + name.len(),
            Self::Spec(source, target) => source.len() + 4 + target.len(),
            Self::Array(values) => {
                let mut len = 0;
                for value in values {
                    len += value.len();
                }
                len
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(value) => write!(f, "{}", value),
            Self::View(source, name) => write!(f, "{} {}", source, name),
            Self::Spec(source, target) => write!(f, "{}.{}", source, target),
            Self::Array(values) => {
                write!(f, "(")?;
                for (i, value) in values.iter().enumerate() {
                    if i == 0 {
                        write!(f, "{}", value)?;
                    } else {
                        write!(f, "{},", value)?;
                    }
                }
                write!(f, ")")
            }
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

    #[test]
    fn array() {
        let spec = Element::array(vec![Box::new(Element::view("table", "t"))]);
        assert_eq!("(table t)", spec.to_string());
    }
}
