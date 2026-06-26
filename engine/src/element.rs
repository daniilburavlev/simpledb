use common::{DbResult, error::DbError};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Element {
    Raw(String),
    Id { source: String, name: String },
    Specifier { source: String, spec: String },
}

impl Element {
    pub(crate) fn source(&self) -> DbResult<&str> {
        match self {
            Self::Raw(value) => Ok(value),
            Self::Specifier { source, .. } => Ok(source),
            e => Err(DbError::UnexpectedToken(e.to_string())),
        }
    }
}

impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(value) => write!(f, "{}", value),
            Self::Id { source, name } => write!(f, "{} {}", source, name),
            Self::Specifier { source, spec } => write!(f, "{}.{}", source, spec),
        }
    }
}
