use std::{collections::HashMap, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    index_mgr::{IndexInfo, IndexMgr},
    layout::Layout,
    schema::Schema,
    stat_mgr::{StatInfo, StatMgr},
    table_mgr::TableMgr,
    view_mgr::ViewMgr,
};

pub struct MetadataMgr {
    table_mgr: Arc<TableMgr>,
    view_mgr: Arc<ViewMgr>,
    stat_mgr: Arc<StatMgr>,
    index_mgr: Arc<IndexMgr>,
}

impl MetadataMgr {
    pub fn new(is_new: bool, tx: &Arc<Transaction>) -> DbResult<Self> {
        let table_mgr = Arc::new(TableMgr::new(is_new, tx)?);
        let view_mgr = Arc::new(ViewMgr::new(is_new, &table_mgr, tx)?);
        let stat_mgr = Arc::new(StatMgr::new(&table_mgr, tx)?);
        let index_mgr = Arc::new(IndexMgr::new(is_new, &table_mgr, &stat_mgr, tx)?);
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
        schema: &Arc<Schema>,
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
        layout: &Arc<Layout>,
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
