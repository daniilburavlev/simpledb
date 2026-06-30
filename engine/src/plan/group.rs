use std::{rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::element::Element;
use crate::schema::SchemaBuilder;
use crate::{
    plan::{Plan, order::SortPlan},
    scan::group::{AggregationFn, GroupByScan},
    schema::Schema,
};

pub struct GroupByPlan {
    plan: Rc<dyn Plan>,
    group_fields: Vec<Element>,
    aggregation_fn: Vec<AggregationFn>,
    schema: Schema,
}

impl GroupByPlan {
    pub fn new(
        tx: &Arc<Transaction>,
        plan: &Rc<dyn Plan>,
        group_fields: Vec<Element>,
        aggregation_fn: Vec<AggregationFn>,
    ) -> DbResult<Self> {
        let mut schema = SchemaBuilder::default();
        let s = plan.schema()?;
        for field in &group_fields {
            schema = schema.add(field.clone(), &s);
        }
        for f in &aggregation_fn {
            schema = schema.add_int_field(f.field_name().clone());
        }
        let schema = schema.build();
        let plan = Rc::new(SortPlan::new(tx, plan, group_fields.clone())?);
        Ok(Self {
            plan,
            group_fields,
            aggregation_fn,
            schema,
        })
    }
}

impl Plan for GroupByPlan {
    fn open(&self) -> DbResult<Rc<dyn crate::scan::Scan>> {
        let s = self.plan.open()?;
        Ok(Rc::new(GroupByScan::new(
            &s,
            self.group_fields.clone(),
            self.aggregation_fn.clone(),
        )?))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        self.plan.blocks_accessed()
    }

    fn records_output(&self) -> DbResult<i32> {
        let mut num_groups = 1;
        for field in &self.group_fields {
            num_groups *= self.plan.distinct_values(field)?;
        }
        Ok(num_groups)
    }

    fn distinct_values(&self, field_name: &Element) -> DbResult<i32> {
        if self.plan.schema()?.has_field(field_name) {
            self.plan.distinct_values(field_name)
        } else {
            self.records_output()
        }
    }

    fn schema(&self) -> DbResult<Schema> {
        Ok(self.schema.clone())
    }
}
