use crate::layout::Layout;
use crate::scan::Scan;
use crate::scan::table::TableScan;
use crate::schema::Schema;
use common::DbResult;
use rand::RngExt;
use std::rc::Rc;
use std::sync::Arc;
use transaction::transaction::Transaction;

#[derive(Clone)]
pub struct TempTable {
    tx: Arc<Transaction>,
    table_name: String,
    layout: Arc<Layout>,
}

impl TempTable {
    pub fn new(tx: &Arc<Transaction>, schema: &Arc<Schema>) -> DbResult<Self> {
        let mut rng = rand::rng();
        let table_num = rng.random::<i32>();
        Ok(Self {
            table_name: format!("temp_{}", table_num),
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
