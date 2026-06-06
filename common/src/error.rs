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
}

impl DbError {
    pub fn lock<T: std::fmt::Display>(e: T) -> Self {
        tracing::info!("{}", e);
        Self::Lock
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
