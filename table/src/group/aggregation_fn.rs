use std::{cmp::Ordering, rc::Rc};

use common::DbResult;

use crate::{constant::Constant, scan::Scan};

#[derive(Clone)]
pub enum AggregationFn {
    MaxFn { field: String, value: Constant },
}

impl AggregationFn {
    pub fn process_first(&mut self, scan: &Rc<dyn Scan>) -> DbResult<()> {
        match self {
            Self::MaxFn { field, value } => {
                *value = scan.get_val(field)?;
            }
        }
        Ok(())
    }

    pub fn process_next(&mut self, scan: &Rc<dyn Scan>) -> DbResult<()> {
        match self {
            Self::MaxFn { field, value } => {
                let new_value = scan.get_val(field)?;
                if new_value.cmp(value) == Ordering::Greater {
                    *value = new_value;
                }
            }
        }
        Ok(())
    }

    pub fn field_name(&self) -> String {
        match self {
            Self::MaxFn { field, .. } => format!("max_of_{}", field),
        }
    }

    pub fn value(&self) -> Constant {
        match self {
            Self::MaxFn { value, .. } => value.clone(),
        }
    }
}
