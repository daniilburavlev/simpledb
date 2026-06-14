use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("file does not exists")]
    FileNotExists,
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),
    #[error("cannot get lock")]
    Lock,
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
}

impl DbError {
    pub fn lock<T: std::fmt::Display>(e: T) -> Self {
        tracing::info!("{}", e);
        Self::Lock
    }

    pub fn field_not_exists(field_name: &str) -> Self {
        Self::FieldNotExists(field_name.to_string())
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
}
