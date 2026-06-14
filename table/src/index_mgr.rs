use std::{collections::HashMap, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    layout::Layout,
    scan::{Scan, table::TableScan},
    schema::Schema,
    stat_mgr::{StatInfo, StatMgr},
    table_mgr::TableMgr,
};

const IDX_TABLE: &str = "idx";
const IDX_NAME: &str = "idx_name";
const TABLE_NAME: &str = "table_name";
const FIELD_NAME: &str = "field_name";
const MAX_LENGTH: u16 = 16;

pub struct IndexInfo {
    idx_name: String,
    field_name: String,
    tx: Arc<Transaction>,
    schema: Arc<Schema>,
    layout: Arc<Layout>,
    stat: StatInfo,
}

impl IndexInfo {
    pub fn new(
        idx_name: String,
        field_name: String,
        schema: &Arc<Schema>,
        tx: &Arc<Transaction>,
        stat: StatInfo,
    ) -> DbResult<Self> {
        todo!()
    }

    fn create_idx_layout() -> DbResult<Layout> {
        let schema = Schema::default();
        schema.add_int_field("block".to_string())?;
        schema.add_int_field("id".to_string())?;
        todo!()
    }

    // pub fn open(&self) -> DbResult<Index> {
    //  todo!()
    //}

    pub fn block_accessed(&self) -> DbResult<i32> {
        let rpb = self.tx.block_size() / self.layout.slotsize() as usize;
        // let num_blocks = self.stat.records_output() / rpb;
        //     HashIndex::searchCost(num_blocks, rpb)
        todo!()
    }

    pub fn records_output(&self) -> i32 {
        self.stat.records_output() / self.stat.distinct_values()
    }

    pub fn distinct_values(&self, field_name: &str) -> i32 {
        if self.field_name == field_name {
            1
        } else {
            self.stat.distinct_values()
        }
    }
}

pub struct IndexMgr {
    layout: Arc<Layout>,
    table_mgr: Arc<TableMgr>,
    stat_mgr: Arc<StatMgr>,
}

impl IndexMgr {
    pub fn new(
        is_new: bool,
        table_mgr: &Arc<TableMgr>,
        stat_mgr: &Arc<StatMgr>,
        tx: &Arc<Transaction>,
    ) -> DbResult<Self> {
        if is_new {
            let schema = Schema::default();
            schema.add_string_field(IDX_NAME.to_string(), MAX_LENGTH)?;
            schema.add_string_field(TABLE_NAME.to_string(), MAX_LENGTH)?;
            schema.add_string_field(FIELD_NAME.to_string(), MAX_LENGTH)?;
            table_mgr.create_table(IDX_TABLE, &Arc::new(schema), tx)?;
        }
        let layout = Arc::new(table_mgr.get_layout(IDX_TABLE, tx)?);
        Ok(Self {
            layout,
            table_mgr: Arc::clone(table_mgr),
            stat_mgr: Arc::clone(stat_mgr),
        })
    }

    pub fn create_index(
        &self,
        idx_name: &str,
        table_name: &str,
        field_name: &str,
        tx: &Arc<Transaction>,
    ) -> DbResult<()> {
        let ts = TableScan::new(tx, IDX_TABLE, &self.layout)?;
        ts.insert()?;
        ts.set_string(IDX_NAME, idx_name)?;
        ts.set_string(TABLE_NAME, table_name)?;
        ts.set_string(FIELD_NAME, field_name)
    }

    pub fn get_index_info(
        &self,
        table_name: &str,
        tx: &Arc<Transaction>,
    ) -> DbResult<HashMap<String, IndexInfo>> {
        let mut result = HashMap::new();
        let ts = TableScan::new(tx, IDX_TABLE, &self.layout)?;
        while ts.next()? {
            if ts.get_string(TABLE_NAME)? == table_name {
                let idx_name = ts.get_string(IDX_NAME)?;
                let field_name = ts.get_string(FIELD_NAME)?;
                let layout = Arc::new(self.table_mgr.get_layout(table_name, tx)?);
                let stat = self.stat_mgr.get_stat_info(table_name, &layout, tx)?;
                // let index = IndexInfo::new(index_name, field_name, layout.schema(), tx, stat)?;
                // result.insert(field_name, index);
                todo!()
            }
        }
        ts.close()?;
        Ok(result)
    }
}
