pub mod error;
pub mod locks;

use crate::error::DbError;

pub type DbResult<T> = Result<T, DbError>;
