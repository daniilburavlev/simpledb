use std::sync::Arc;

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    layout::Layout,
    metadata_mgr::MetadataMgr,
    scan::{Scan, table::TableScan},
    schema::Schema,
    stat_mgr::StatInfo,
};

pub struct TablePlan {
    tx: Arc<Transaction>,
    table: String,
    layout: Arc<Layout>,
    stat: StatInfo,
}

impl TablePlan {
    pub fn new(tx: &Arc<Transaction>, table: String, md: &MetadataMgr) -> DbResult<Self> {
        let layout = Arc::new(md.get_layout(&table, tx)?);
        let stat = md.get_stat_info(&table, &layout, tx)?;
        Ok(Self {
            tx: Arc::clone(tx),
            table,
            layout,
            stat,
        })
    }
}

impl TablePlan {
    pub fn open(&self) -> DbResult<Box<dyn Scan>> {
        Ok(Box::new(TableScan::new(
            &self.tx,
            &self.table,
            &self.layout,
        )?))
    }

    pub fn blocks_accessed(&self) -> DbResult<i32> {
        Ok(self.stat.block_accessed())
    }

    pub fn records_output(&self) -> DbResult<i32> {
        Ok(self.stat.records_output())
    }

    pub fn distinct_values(&self, _: &str) -> DbResult<i32> {
        Ok(self.stat.distinct_values())
    }

    pub fn schema(&self) -> DbResult<Arc<Schema>> {
        Ok(self.layout.schema())
    }
}
