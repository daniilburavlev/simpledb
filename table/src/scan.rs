use common::DbResult;

use crate::{constant::Constant, rid::RID};

pub mod product_scan;
pub mod project_scan;
pub mod select_scan;
pub mod table_scan;

pub trait Scan {
    fn before_first(&self) -> DbResult<()>;

    fn next(&self) -> DbResult<bool>;

    fn get_i32(&self, field_name: &str) -> DbResult<i32>;

    fn get_string(&self, field_name: &str) -> DbResult<String>;

    fn get_val(&self, field_name: &str) -> DbResult<Constant>;

    fn has_field(&self, field_name: &str) -> DbResult<bool>;

    fn close(&self) -> DbResult<()>;
}

pub trait UpdateScan: Scan {
    fn set_i32(&self, field_name: &str, value: i32) -> DbResult<()>;

    fn set_string(&self, field_name: &str, value: &str) -> DbResult<()>;

    fn set_val(&self, field_name: &str, value: Constant) -> DbResult<()>;

    fn insert(&self) -> DbResult<()>;

    fn delete(&self) -> DbResult<()>;

    fn get_rid(&self) -> DbResult<RID>;

    fn move_to_rid(&self, rid: RID) -> DbResult<()>;
}
