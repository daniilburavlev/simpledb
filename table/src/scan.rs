use common::{DbResult, error::DbError};
use std::sync::Arc;

use crate::schema::Schema;
use crate::{constant::Constant, rid::RID};

pub mod chunk;
pub mod index;
pub mod product;
pub mod project;
pub mod select;
pub mod table;

pub trait Scan {
    fn before_first(&self) -> DbResult<()>;

    fn next(&self) -> DbResult<bool>;

    fn get_i32(&self, field_name: &str) -> DbResult<i32>;

    fn get_string(&self, field_name: &str) -> DbResult<String>;

    fn get_val(&self, field_name: &str) -> DbResult<Constant>;

    fn has_field(&self, field_name: &str) -> DbResult<bool>;

    fn close(&self) -> DbResult<()>;

    fn schema(&self) -> DbResult<Arc<Schema>>;

    fn set_i32(&self, _: &str, _: i32) -> DbResult<()> {
        Err(DbError::other("cannot set integer"))
    }

    fn set_string(&self, _: &str, _: &str) -> DbResult<()> {
        Err(DbError::other("cannot set string"))
    }

    fn set_val(&self, _: &str, _: Constant) -> DbResult<()> {
        Err(DbError::other("cannot set value"))
    }

    fn insert(&self) -> DbResult<()> {
        Err(DbError::other("cannot insert"))
    }

    fn delete(&self) -> DbResult<()> {
        Err(DbError::other("cannot delete"))
    }

    fn get_rid(&self) -> DbResult<RID> {
        Err(DbError::other("cannot get rid"))
    }

    fn move_to_rid(&self, _: RID) -> DbResult<()> {
        Err(DbError::other("cannot update"))
    }

    fn save_position(&self) -> DbResult<()> {
        Err(DbError::other("cannot save position"))
    }
}
