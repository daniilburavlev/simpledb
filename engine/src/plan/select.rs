use std::rc::Rc;

use common::DbResult;

use crate::element::Element;
use crate::{
    plan::Plan,
    predicate::Predicate,
    scan::{Scan, select::SelectScan},
    schema::Schema,
};

pub(crate) struct SelectPlan {
    plan: Rc<dyn Plan>,
    predicate: Predicate,
}

impl SelectPlan {
    pub(crate) fn new(plan: Rc<dyn Plan>, predicate: Predicate) -> Self {
        Self { plan, predicate }
    }
}

impl Plan for SelectPlan {
    fn open(&self) -> DbResult<Rc<dyn Scan>> {
        let s = self.plan.open()?;
        Ok(Rc::new(SelectScan::new(s, self.predicate.clone())))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        self.plan.blocks_accessed()
    }

    fn records_output(&self) -> DbResult<i32> {
        Ok(self.plan.records_output()? / self.predicate.reduction_factor(&self.plan)?)
    }

    fn distinct_values(&self, field_name: &Element) -> common::DbResult<i32> {
        if self.predicate.equates_with_constant(field_name)?.is_some() {
            return Ok(1);
        } else {
            if let Some(second_field) = self.predicate.equates_with_field(field_name)? {
                return Ok(self
                    .plan
                    .distinct_values(field_name)?
                    .min(self.plan.distinct_values(&second_field)?));
            }
        }
        self.plan.distinct_values(field_name)
    }

    fn schema(&self) -> DbResult<Schema> {
        self.plan.schema()
    }
}
