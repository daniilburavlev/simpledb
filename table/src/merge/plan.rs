use std::{rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{merge::scan::MergeJoinScan, plan::Plan, schema::Schema, sort::plan::SortPlan};

pub struct MergeJoinPlan {
    p1: Rc<dyn Plan>,
    p2: Rc<dyn Plan>,
    field_name1: String,
    field_name2: String,
    schema: Arc<Schema>,
}

impl MergeJoinPlan {
    pub fn new(
        tx: &Arc<Transaction>,
        p1: &Rc<dyn Plan>,
        p2: &Rc<dyn Plan>,
        field_name1: &str,
        field_name2: &str,
    ) -> DbResult<Self> {
        let p1 = Rc::new(SortPlan::new(tx, p1, vec![field_name1.to_string()])?);
        let p2 = Rc::new(SortPlan::new(tx, p2, vec![field_name2.to_string()])?);
        let schema = Arc::new(Schema::default());
        Ok(Self {
            p1,
            p2,
            field_name1: field_name1.to_string(),
            field_name2: field_name2.to_string(),
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
            &self.field_name1,
            &self.field_name2,
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

    fn distinct_values(&self, field_name: &str) -> DbResult<i32> {
        if self.p1.schema()?.has_field(field_name)? {
            self.p1.distinct_values(field_name)
        } else {
            self.p2.distinct_values(field_name)
        }
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        Ok(Arc::clone(&self.schema))
    }
}
