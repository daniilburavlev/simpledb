use std::{collections::HashMap, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    layout::Layout,
    mgr::{
        index::{IndexInfo, IndexMgr},
        stat::{StatInfo, StatMgr},
        table::TableMgr,
        view::ViewMgr,
    },
    schema::Schema,
};

pub struct MetadataMgr {
    table_mgr: TableMgr,
    view_mgr: ViewMgr,
    stat_mgr: StatMgr,
    index_mgr: IndexMgr,
}

impl MetadataMgr {
    pub fn new(is_new: bool, tx: &Arc<Transaction>) -> DbResult<Self> {
        let table_mgr = TableMgr::new(is_new, tx)?;
        let view_mgr = ViewMgr::new(is_new, table_mgr.clone(), tx)?;
        let stat_mgr = StatMgr::new(table_mgr.clone(), tx)?;
        let index_mgr = IndexMgr::new(is_new, table_mgr.clone(), stat_mgr.clone(), tx)?;
        Ok(Self {
            table_mgr,
            view_mgr,
            stat_mgr,
            index_mgr,
        })
    }

    pub fn create_table(
        &self,
        table_name: &str,
        schema: Schema,
        tx: &Arc<Transaction>,
    ) -> DbResult<()> {
        self.table_mgr.create_table(table_name, schema, tx)
    }

    pub fn get_layout(&self, table_name: &str, tx: &Arc<Transaction>) -> DbResult<Layout> {
        self.table_mgr.get_layout(table_name, tx)
    }

    pub fn create_view(
        &self,
        view_name: &str,
        view_def: &str,
        tx: &Arc<Transaction>,
    ) -> DbResult<()> {
        self.view_mgr.create_view(view_name, view_def, tx)
    }

    pub fn get_view_def(&self, view_name: &str, tx: &Arc<Transaction>) -> DbResult<Option<String>> {
        self.view_mgr.get_view_def(view_name, tx)
    }

    pub fn create_index(
        &self,
        idx_name: &str,
        table_name: &str,
        field_name: &str,
        tx: &Arc<Transaction>,
    ) -> DbResult<()> {
        self.index_mgr
            .create_index(idx_name, table_name, field_name, tx)
    }

    pub fn get_stat_info(
        &self,
        table: &str,
        layout: Layout,
        tx: &Arc<Transaction>,
    ) -> DbResult<StatInfo> {
        self.stat_mgr.get_stat_info(table, layout, tx)
    }

    pub fn get_index_info(
        &self,
        table_name: &str,
        tx: &Arc<Transaction>,
    ) -> DbResult<HashMap<String, IndexInfo>> {
        self.index_mgr.get_index_info(table_name, tx)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn metadata() {}
}
