use common::DbResult;

use crate::{element::Element, plan::Plan, predicate::Predicate, scan::Scan, schema::Schema};

pub(crate) struct SelectPlan {
    plan: Box<Plan>,
    predicate: Predicate,
}

impl SelectPlan {
    pub(crate) fn new(plan: Box<Plan>, predicate: Predicate) -> Self {
        Self { plan, predicate }
    }

    pub(crate) fn open(&self) -> DbResult<Scan> {
        let s = self.plan.open()?;
        Ok(Scan::select(Box::new(s), self.predicate.clone()))
    }

    pub(crate) fn blocks_accessed(&self) -> DbResult<i32> {
        self.plan.blocks_accessed()
    }

    pub(crate) fn records_output(&self) -> DbResult<i32> {
        Ok(self.plan.records_output()? / self.predicate.reduction_factor(&self.plan)?)
    }

    pub(crate) fn distinct_values(&self, field_name: &Element) -> common::DbResult<i32> {
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

    pub(crate) fn schema(&self) -> DbResult<Schema> {
        self.plan.schema()
    }
}
