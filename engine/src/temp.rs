use crate::layout::Layout;
use crate::scan::Scan;
use crate::scan::table::TableScan;
use crate::schema::Schema;
use common::DbResult;
use rand::RngExt;
use std::rc::Rc;
use std::sync::{atomic, Arc};
use std::sync::atomic::AtomicI32;
use transaction::transaction::Transaction;

static TABLE_NUM_COUNTER: AtomicI32 = AtomicI32::new(0);

fn next_table_num() -> i32 {
    TABLE_NUM_COUNTER.fetch_add(1, atomic::Ordering::SeqCst)
}

#[derive(Clone)]
pub struct TempTable {
    tx: Arc<Transaction>,
    table_name: String,
    layout: Arc<Layout>,
}

impl TempTable {
    pub fn new(tx: &Arc<Transaction>, schema: &Arc<Schema>) -> DbResult<Self> {
        Ok(Self {
            table_name: format!("temp_{}", next_table_num()),
            tx: Arc::clone(tx),
            layout: Arc::new(Layout::new(schema)?),
        })
    }

    pub fn open(&self) -> DbResult<Rc<dyn Scan>> {
        Ok(Rc::new(TableScan::new(
            &self.tx,
            &self.table_name,
            &self.layout,
        )?))
    }

    pub(crate) fn table_name(&self) -> String {
        self.table_name.clone()
    }

    pub(crate) fn layout(&self) -> Arc<Layout> {
        self.layout.clone()
    }
}
