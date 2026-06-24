use std::{rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    plan::{Plan, materialize::MaterializePlan},
    scan::{Scan, multibuffer::MultiBufferProductScan},
    schema::Schema,
    temp::TempTable,
};

pub(crate) struct MultiBufferProductPlan {
    tx: Arc<Transaction>,
    left: Rc<dyn Plan>,
    right: Rc<dyn Plan>,
    schema: Arc<Schema>,
}

impl MultiBufferProductPlan {
    pub(crate) fn new(
        tx: &Arc<Transaction>,
        left: &Rc<dyn Plan>,
        right: &Rc<dyn Plan>,
    ) -> DbResult<Self> {
        let schema = Arc::new(Schema::default());
        let s1 = left.schema()?;
        let s2 = right.schema()?;
        schema.add_all(&s1)?;
        schema.add_all(&s2)?;
        let plan = Self {
            tx: Arc::clone(tx),
            left: Rc::clone(left),
            right: Rc::clone(right),
            schema,
        };
        Ok(plan)
    }

    fn copy_records_from(&self, p: &Rc<dyn Plan>) -> DbResult<TempTable> {
        let source = p.open()?;
        let schema = p.schema()?;
        let tt = TempTable::new(&self.tx, &schema)?;
        let dest = tt.open()?;
        while source.next()? {
            dest.insert()?;
            for (field, _) in schema.fields()? {
                let value = source.get_val(&field)?;
                dest.set_val(&field, value)?;
            }
        }
        source.close()?;
        dest.close()?;
        Ok(tt)
    }
}

impl Plan for MultiBufferProductPlan {
    fn open(&self) -> DbResult<Rc<dyn Scan>> {
        let left = self.left.open()?;
        let t = self.copy_records_from(&self.right)?;
        Ok(Rc::new(MultiBufferProductScan::new(
            &self.tx,
            &left,
            &t.table_name(),
            &t.layout(),
        )?))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        let available = self.tx.available_buffs()? as i32;
        let size = MaterializePlan::new(&self.right, &self.tx).blocks_accessed()?;
        let num_chunks = size / available;
        Ok(self.right.blocks_accessed()? + self.left.blocks_accessed()? * num_chunks)
    }

    fn records_output(&self) -> DbResult<i32> {
        Ok(self.left.records_output()? * self.right.records_output()?)
    }

    fn distinct_values(&self, field_name: &str) -> DbResult<i32> {
        if self.left.schema()?.has_field(field_name)? {
            self.left.distinct_values(field_name)
        } else {
            self.right.distinct_values(field_name)
        }
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        Ok(Arc::clone(&self.schema))
    }
}
