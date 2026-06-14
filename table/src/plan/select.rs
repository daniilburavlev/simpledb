use std::sync::Arc;

use common::DbResult;

use crate::{plan::Plan, predicate::Predicate, scan::select_scan::SelectScan, schema::Schema};

pub struct SelectPlan {
    plan: Box<Plan>,
    predicate: Predicate,
}

impl SelectPlan {
    pub fn new(plan: Box<Plan>, predicate: Predicate) -> Self {
        Self { plan, predicate }
    }
}

impl SelectPlan {
    pub fn open(&self) -> DbResult<SelectScan> {
        let s = self.plan.open()?;
        Ok(SelectScan::new(Box::new(s), self.predicate.clone()))
    }

    pub fn blocks_accessed(&self) -> DbResult<i32> {
        self.plan.blocks_accessed()
    }

    pub fn records_output(&self) -> DbResult<i32> {
        Ok(self.plan.records_output()? / self.predicate.reduction_factor(&self.plan)?)
    }

    pub fn distinct_values(&self, field_name: &str) -> common::DbResult<i32> {
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

    pub fn schema(&self) -> DbResult<Arc<Schema>> {
        self.plan.schema()
    }
}
