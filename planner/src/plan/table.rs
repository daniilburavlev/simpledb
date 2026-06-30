use std::sync::Arc;

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    element::Element,
    layout::Layout,
    mgr::{metadata::MetadataMgr, stat::StatInfo},
    scan::Scan,
    schema::Schema,
};

pub(crate) struct TablePlan {
    tx: Arc<Transaction>,
    table: String,
    layout: Layout,
    stat: StatInfo,
}

impl TablePlan {
    pub(crate) fn new(tx: &Arc<Transaction>, table: String, md: &MetadataMgr) -> DbResult<Self> {
        let layout = md.get_layout(&table, tx)?;
        let stat = md.get_stat_info(&table, layout.clone(), tx)?;
        Ok(Self {
            tx: Arc::clone(tx),
            table,
            layout,
            stat,
        })
    }

    pub(crate) fn open(&self) -> DbResult<Scan> {
        Scan::table(&self.tx, &self.table, self.layout.clone())
    }

    pub(crate) fn blocks_accessed(&self) -> DbResult<i32> {
        Ok(self.stat.block_accessed())
    }

    pub(crate) fn records_output(&self) -> DbResult<i32> {
        Ok(self.stat.records_output())
    }

    pub(crate) fn distinct_values(&self, _: &Element) -> DbResult<i32> {
        Ok(self.stat.distinct_values())
    }

    pub(crate) fn schema(&self) -> Schema {
        self.layout.schema().clone()
    }
}
