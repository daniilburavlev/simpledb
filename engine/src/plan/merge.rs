use std::{rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::element::Element;
use crate::schema::SchemaBuilder;
use crate::{
    plan::{Plan, order::SortPlan},
    scan::merge::MergeJoinScan,
    schema::Schema,
};

pub struct MergeJoinPlan {
    p1: Rc<dyn Plan>,
    p2: Rc<dyn Plan>,
    field_name1: Element,
    field_name2: Element,
    schema: Schema,
}

#[allow(dead_code)]
impl MergeJoinPlan {
    pub fn new(
        tx: &Arc<Transaction>,
        p1: &Rc<dyn Plan>,
        p2: &Rc<dyn Plan>,
        field_name1: Element,
        field_name2: Element,
    ) -> DbResult<Self> {
        let p1 = Rc::new(SortPlan::new(tx, p1, vec![field_name1.clone()])?);
        let p2 = Rc::new(SortPlan::new(tx, p2, vec![field_name2.clone()])?);
        let schema = SchemaBuilder::default().build();
        Ok(Self {
            p1,
            p2,
            field_name1,
            field_name2,
            schema,
        })
    }
}

impl Plan for MergeJoinPlan {
    fn open(&self) -> DbResult<Rc<dyn crate::scan::Scan>> {
        let s1 = self.p1.open()?;
        let s2 = self.p2.open()?;
        Ok(Rc::new(MergeJoinScan::new(
            &s1,
            &s2,
            self.field_name1.clone(),
            self.field_name2.clone(),
        )?))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        Ok(self.p1.blocks_accessed()? + self.p2.blocks_accessed()?)
    }

    fn records_output(&self) -> DbResult<i32> {
        let max_val = self
            .p1
            .distinct_values(&self.field_name1)?
            .max(self.p2.distinct_values(&self.field_name2)?);
        Ok(self.p1.records_output()? * self.p2.records_output()? / max_val)
    }

    fn distinct_values(&self, field_name: &Element) -> DbResult<i32> {
        if self.p1.schema()?.has_field(field_name) {
            self.p1.distinct_values(field_name)
        } else {
            self.p2.distinct_values(field_name)
        }
    }

    fn schema(&self) -> DbResult<Schema> {
        Ok(self.schema.clone())
    }
}
