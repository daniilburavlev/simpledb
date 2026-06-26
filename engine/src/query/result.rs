use std::rc::Rc;

use crate::scan::Scan;

pub struct QueryResult {
    scan: Rc<dyn Scan>,
}

impl QueryResult {}
