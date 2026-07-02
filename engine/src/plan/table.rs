use std::{rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::element::Element;
use crate::{
    layout::Layout,
    metadata_mgr::MetadataMgr,
    plan::Plan,
    scan::{Scan, table::TableScan},
    schema::Schema,
    stat_mgr::StatInfo,
};

pub struct TablePlan {
    tx: Arc<Transaction>,
    table: Element,
    layout: Layout,
    stat: StatInfo,
}

impl TablePlan {
    pub fn new(tx: &Arc<Transaction>, table: String, md: &MetadataMgr) -> DbResult<Self> {
        let layout = md.get_layout(&table, tx)?;
        let stat = md.get_stat_info(&table, layout.clone(), tx)?;
        Ok(Self {
            tx: Arc::clone(tx),
            table: Element::Raw(table),
            layout,
            stat,
        })
    }
}

impl Plan for TablePlan {
    fn open(&self) -> DbResult<Rc<dyn Scan>> {
        Ok(Rc::new(TableScan::new(
            &self.tx,
            self.table.as_raw()?,
            self.layout.clone(),
        )?))
    }

    fn blocks_accessed(&self) -> DbResult<i32> {
        Ok(self.stat.block_accessed())
    }

    fn records_output(&self) -> DbResult<i32> {
        Ok(self.stat.records_output())
    }

    fn distinct_values(&self, _: &Element) -> DbResult<i32> {
        Ok(self.stat.distinct_values())
    }

    fn schema(&self) -> DbResult<Schema> {
        Ok(self.layout.schema().clone())
    }
}
