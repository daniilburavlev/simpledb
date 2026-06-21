use crate::layout::Layout;
use crate::scan::Scan;
use crate::scan::table::TableScan;
use crate::schema::Schema;
use common::DbResult;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::SeqCst;
use transaction::transaction::Transaction;

mod meterialize_plan;

#[derive(Clone)]
pub struct NextTableNum(Arc<AtomicI32>);

impl NextTableNum {
    pub fn next(&self) -> i32 {
        self.0.fetch_add(1, SeqCst)
    }
}

impl Default for NextTableNum {
    fn default() -> Self {
        NextTableNum(Arc::new(AtomicI32::new(0)))
    }
}

pub struct TempTable {
    tx: Arc<Transaction>,
    table_name: String,
    layout: Arc<Layout>,
}

impl TempTable {
    pub fn new(
        next_table_num: &NextTableNum,
        tx: &Arc<Transaction>,
        schema: &Arc<Schema>,
    ) -> DbResult<Self> {
        Ok(Self {
            table_name: format!("temp_{}", next_table_num.next()),
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

    fn table_name(&self) -> String {
        self.table_name.clone()
    }

    fn layout(&self) -> Arc<Layout> {
        self.layout.clone()
    }
}
