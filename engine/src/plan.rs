use std::rc::Rc;

use crate::element::Element;
use crate::{scan::Scan, schema::Schema};
use common::DbResult;
use common::error::DbError;

pub(crate) mod group;
pub mod index;
pub(crate) mod materialize;
pub(crate) mod merge;
pub(crate) mod multibuffer;
pub(crate) mod order;
pub mod product;
pub mod project;
pub mod select;
pub mod table;

pub trait Plan {
    fn open(&self) -> DbResult<Rc<dyn Scan>>;

    fn blocks_accessed(&self) -> DbResult<i32>;

    fn records_output(&self) -> DbResult<i32>;

    fn distinct_values(&self, field_name: &Element) -> DbResult<i32>;

    fn schema(&self) -> DbResult<Schema>;
}
