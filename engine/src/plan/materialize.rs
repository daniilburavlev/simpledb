use crate::layout::Layout;
use crate::plan::Plan;
use crate::scan::Scan;
use crate::schema::Schema;
use crate::temp::TempTable;
use common::DbResult;
use std::rc::Rc;
use std::sync::Arc;
use transaction::transaction::Transaction;

pub struct MaterializePlan {
    source: Rc<dyn Plan>,
    tx: Arc<Transaction>,
}

impl MaterializePlan {
    pub fn new(source: &Rc<dyn Plan>, tx: &Arc<Transaction>) -> Self {
        Self {
            source: Rc::clone(source),
            tx: Arc::clone(tx),
        }
    }
}

impl Plan for MaterializePlan {
    fn open(&self) -> DbResult<Rc<dyn Scan>> {
        let schema = self.source.schema()?;
        let temp = TempTable::new(&self.tx, &schema)?;
        let source = self.source.open()?;
        let dest = temp.open()?;
        while source.next()? {
            dest.insert()?;
            for (field, _) in schema.fields()? {
                dest.set_val(&field, source.get_val(&field)?)?;
            }
        }
        source.close()?;
        dest.before_first()?;
        Ok(dest)
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        let layout = Layout::new(&self.source.schema()?)?;
        let rpd = self.tx.block_size() / layout.slotsize();
        Ok(self.source.records_output()? / rpd)
    }

    fn records_output(&self) -> DbResult<i32> {
        self.source.records_output()
    }

    fn distinct_values(&self, field_name: &str) -> DbResult<i32> {
        self.source.distinct_values(field_name)
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        self.source.schema()
    }
}
