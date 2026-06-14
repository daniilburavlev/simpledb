use std::sync::Arc;

use common::DbResult;

use crate::scan::Scan;

pub trait Plan {
    fn open(&self) -> DbResult<Box<dyn Scan>>;

    fn blocks_accessed(&self) -> DbResult<i32>;

    fn records_output(&self) -> DbResult<i32>;

    fn distinct_values(&self, field_name: &str) -> DbResult<i32>;

    fn schema(&self) -> DbResult<Arc<Schema>>;
}
