use std::{collections::HashMap, rc::Rc, sync::Arc};

use common::DbResult;
use common::error::DbError;
use transaction::transaction::Transaction;

use crate::{
    index::{Index, b_tree::BTreeIndex, btree_page::VALUE},
    layout::Layout,
    scan::{Scan, table::TableScan},
    schema::Schema,
    stat_mgr::{StatInfo, StatMgr},
    table_mgr::TableMgr,
};
use crate::index::btree_page::{BLOCK, ID};

const IDX_TABLE: &str = "idx";
const IDX_NAME: &str = "idx_name";
const TABLE_NAME: &str = "table_name";
const FIELD_NAME: &str = "field_name";
const MAX_LENGTH: i32 = 16;

pub struct IndexInfo {
    idx_name: String,
    field_name: String,
    tx: Arc<Transaction>,
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
        let layout = Arc::new(Self::create_idx_layout(&field_name, schema)?);
        Ok(Self {
            idx_name,
            field_name,
            tx: Arc::clone(tx),
            layout,
            stat,
        })
    }

    fn create_idx_layout(field_name: &str, table_schema: &Arc<Schema>) -> DbResult<Layout> {
        let schema = Arc::new(Schema::default());
        schema.add_int_field(BLOCK.to_string())?;
        schema.add_int_field(ID.to_string())?;
        if let Some(info) = table_schema.info(field_name)? {
            schema.add_field(VALUE.to_string(), info)?;
        }
        Layout::new(&schema)
    }

    pub fn open(&self) -> DbResult<Rc<dyn Index>> {
        Ok(Rc::new(BTreeIndex::new(
            &self.tx,
            &self.idx_name,
            &self.layout,
        )?))
    }

    pub fn block_accessed(&self) -> DbResult<i32> {
        let rpb = self.tx.block_size() as i32 / self.layout.slotsize() as i32;
        let num_blocks = self.stat.records_output() / rpb;
        Ok(num_blocks)
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
        if layout.schema().fields()?.is_empty() {
            return Err(DbError::other("cannot initialize inner index table"));
        }
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
        ts.set_string(FIELD_NAME, field_name)?;
        ts.close()
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
                let index =
                    IndexInfo::new(idx_name, field_name.clone(), &layout.schema(), tx, stat)?;
                result.insert(field_name, index);
            }
        }
        ts.close()?;
        Ok(result)
    }
}
