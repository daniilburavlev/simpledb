use common::{DbResult, error::DbError};
use std::sync::Arc;

use crate::schema::Schema;
use crate::{value::Value, rid::RID};
use crate::element::Element;

pub mod chunk;
pub(crate) mod group;
pub mod index;
pub(crate) mod merge;
pub(crate) mod multibuffer;
pub mod product;
pub mod project;
pub mod select;
pub(crate) mod order;
pub mod table;

pub trait Scan {
    fn before_first(&self) -> DbResult<()>;

    fn next(&self) -> DbResult<bool>;

    fn get_i32(&self, field_name: &Element) -> DbResult<i32>;

    fn get_string(&self, field_name: &Element) -> DbResult<String>;

    fn get_val(&self, field_name: &Element) -> DbResult<Value>;

    fn has_field(&self, field_name: &Element) -> DbResult<bool>;

    fn close(&self) -> DbResult<()>;

    fn schema(&self) -> DbResult<Schema>;

    fn set_i32(&self, _: &Element, _: i32) -> DbResult<()> {
        Err(DbError::other("cannot set integer"))
    }

    fn set_string(&self, _: &Element, _: &str) -> DbResult<()> {
        Err(DbError::other("cannot set string"))
    }

    fn set_val(&self, _: &Element, _: Value) -> DbResult<()> {
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
