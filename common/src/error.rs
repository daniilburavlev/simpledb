use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("file does not exists")]
    FileNotExists,
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),
    #[error("cannot get lock")]
    MutexLock,
    #[error("empty buffer block")]
    EmtyBufferBlock,
    #[error("buffer aborted")]
    BufferAbort,
    #[error("decoding error")]
    Decoding,
    #[error("lock abort")]
    LockAbort,
    #[error("lock timeout")]
    LockTimeout,
    #[error("unexisted buffer")]
    UnexistedBuffer,
    #[error("field '{0}' not exists")]
    FieldNotExists(String),
    #[error("unknown type")]
    UnknownType,
    #[error("EOF: {0}")]
    EOF(String),
    #[error("bad syntax")]
    BadSyntax,
    #[error("{0}")]
    Other(String),
    #[error("invalid values amount")]
    InvalidValuesAmount,
    #[error("invalid field type")]
    InvalidFieldType,
    #[error("{0}")]
    ToInt(#[from] std::num::TryFromIntError),
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("max size ({0}) exceeded: {1}")]
    MaxSize(usize, usize),
}

impl DbError {
    pub fn lock<T: std::fmt::Display>(e: T) -> Self {
        tracing::info!("{}", e);
        Self::MutexLock
    }

    pub fn field_not_exists(field_name: &str) -> Self {
        Self::FieldNotExists(field_name.to_string())
    }

    pub fn other(msg: &str) -> Self {
        Self::Other(msg.to_string())
    }

    pub fn unexpected_token(msg: &str) -> Self {
        Self::UnexpectedToken(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::DbError;

    #[test]
    fn lock() {
        let error = DbError::lock("hello");
        assert_eq!(error.to_string(), "cannot get lock");
    }

    #[test]
    fn field_not_exists() {
        let error = DbError::field_not_exists("name");
        assert_eq!(error.to_string(), "field 'name' not exists");
    }

    #[test]
    fn other() {
        let error = DbError::other("other");
        assert_eq!(error.to_string(), "other");
    }
}
