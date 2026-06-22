use std::{cmp::Ordering, rc::Rc};

use common::DbResult;

use crate::scan::Scan;

#[derive(Clone)]
pub struct RecordComparator {
    fields: Vec<String>,
}

impl RecordComparator {
    pub fn new(fields: Vec<String>) -> Self {
        Self { fields }
    }

    pub fn compare(&self, s1: &Rc<dyn Scan>, s2: &Rc<dyn Scan>) -> DbResult<Ordering> {
        for field in &self.fields {
            let val1 = s1.get_val(field)?;
            let val2 = s2.get_val(field)?;
            let result = val1.cmp(&val2);
            if result != Ordering::Equal {
                return Ok(result);
            }
        }
        Ok(Ordering::Equal)
    }
}
