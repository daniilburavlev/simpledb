use std::{rc::Rc, sync::Arc};

use common::DbResult;

use crate::{scan::Scan, schema::Schema};

pub mod product;
pub mod project;
pub mod select;
pub mod table;

pub trait Plan {
    fn open(&self) -> DbResult<Rc<dyn Scan>>;

    fn blocks_accessed(&self) -> DbResult<i32>;

    fn records_output(&self) -> DbResult<i32>;

    fn distinct_values(&self, field_name: &str) -> DbResult<i32>;

    fn schema(&self) -> DbResult<Arc<Schema>>;
}
