use common::DbResult;
use common::error::DbError;
use std::rc::Rc;
use std::{collections::HashMap, sync::Arc};
use transaction::transaction::Transaction;

use crate::element::Element;
use crate::index::Index;
use crate::index::b_tree::BTreeIndex;
use crate::layout::Layout;
use crate::scan::Scan;
use crate::scan::table::TableScan;
use crate::schema::{Schema, SchemaBuilder};
use crate::stat_mgr::{StatInfo, StatMgr};
use crate::table_mgr::TableMgr;

const IDX_TABLE: &str = "idx";
const IDX_NAME: &str = "idx_name";
const TABLE_NAME: &str = "table_name";
const FIELD_NAME: &str = "field_name";
const MAX_LENGTH: i32 = 16;

#[derive(Clone)]
pub struct IndexInfo {
    idx_name: String,
    field_name: Element,
    tx: Arc<Transaction>,
    slot_size: i32,
    stat: StatInfo,
}

impl IndexInfo {
    pub fn new(
        idx_name: String,
        field_name: Element,
        schema: &Schema,
        tx: &Arc<Transaction>,
        stat: StatInfo,
    ) -> DbResult<Self> {
        let Some(info) = schema.info(&field_name) else {
            return Err(DbError::FieldNotExists(field_name.to_string()));
        };
        Ok(Self {
            idx_name,
            slot_size: info.length(),
            field_name,
            tx: Arc::clone(tx),
            stat,
        })
    }

    pub fn open(&self) -> DbResult<Rc<dyn Index>> {
        Ok(Rc::new(BTreeIndex::new(&self.idx_name, &self.tx)?))
    }

    pub fn block_accessed(&self) -> DbResult<i32> {
        let rpb = self.tx.block_size() / self.slot_size;
        let num_blocks = self.stat.records_output() / rpb;
        Ok(num_blocks)
    }

    pub fn records_output(&self) -> i32 {
        self.stat.records_output() / self.stat.distinct_values()
    }

    pub fn distinct_values(&self, field_name: &Element) -> i32 {
        if self.field_name == *field_name {
            1
        } else {
            self.stat.distinct_values()
        }
    }
}

#[derive(Clone)]
pub struct IndexMgr {
    layout: Layout,
    table_mgr: TableMgr,
    stat_mgr: StatMgr,
}

impl IndexMgr {
    pub fn new(
        is_new: bool,
        table_mgr: TableMgr,
        stat_mgr: StatMgr,
        tx: &Arc<Transaction>,
    ) -> DbResult<Self> {
        if is_new {
            let schema = SchemaBuilder::new(Element::raw(IDX_TABLE))
                .add_string_field(Element::raw(IDX_NAME), MAX_LENGTH)
                .add_string_field(Element::raw(TABLE_NAME), MAX_LENGTH)
                .add_string_field(Element::raw(FIELD_NAME), MAX_LENGTH)
                .build();
            table_mgr.create_table(IDX_TABLE, schema, tx)?;
        }
        let layout = table_mgr.get_layout(IDX_TABLE, tx)?;
        if layout.schema().fields().is_empty() {
            return Err(DbError::other("cannot initialize inner index table"));
        }
        Ok(Self {
            layout,
            table_mgr,
            stat_mgr,
        })
    }

    pub fn create_index(
        &self,
        idx_name: &str,
        table_name: &str,
        field_name: &str,
        tx: &Arc<Transaction>,
    ) -> DbResult<()> {
        let ts = TableScan::new(tx, IDX_TABLE, self.layout.clone())?;
        ts.insert()?;
        ts.set_string(&Element::raw(IDX_NAME), idx_name)?;
        ts.set_string(&Element::raw(TABLE_NAME), table_name)?;
        ts.set_string(&Element::raw(FIELD_NAME), field_name)?;
        ts.close()
    }

    pub fn get_index_info(
        &self,
        table_name: &str,
        tx: &Arc<Transaction>,
    ) -> DbResult<HashMap<Element, IndexInfo>> {
        let mut result = HashMap::new();
        let ts = TableScan::new(tx, IDX_TABLE, self.layout.clone())?;
        while ts.next()? {
            if ts.get_string(&Element::raw(TABLE_NAME))? == table_name {
                let idx_name = ts.get_string(&Element::raw(IDX_NAME))?;
                let field_name = ts.get_string(&Element::raw(FIELD_NAME))?;
                let layout = self.table_mgr.get_layout(table_name, tx)?;
                let stat = self
                    .stat_mgr
                    .get_stat_info(table_name, layout.clone(), tx)?;
                let index = IndexInfo::new(
                    idx_name,
                    Element::raw(&field_name),
                    layout.schema(),
                    tx,
                    stat,
                )?;
                result.insert(Element::Raw(field_name), index);
            }
        }
        ts.close()?;
        Ok(result)
    }
}
