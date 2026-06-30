use std::sync::Arc;

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{element::Element, scan::Scan, schema::SchemaBuilder};
use crate::scan::table::TableScan;
use crate::table_mgr::TableMgr;

const MAX_NAME: i32 = 16;
const MAX_VIEW_DEF: i32 = 100;

const VIEW_NAME: &str = "view";
const VIEW_DEF: &str = "view_def";
const VIEW_TABLE: &str = "sp_view";

pub struct ViewMgr {
    table_mgr: TableMgr,
}

impl ViewMgr {
    pub fn new(is_new: bool, table_mgr: TableMgr, tx: &Arc<Transaction>) -> DbResult<Self> {
        if is_new {
            let schema = SchemaBuilder::default()
                .add_string_field(Element::raw(VIEW_NAME), MAX_NAME)
                .add_string_field(Element::raw(VIEW_DEF), MAX_VIEW_DEF)
                .build();
            table_mgr.create_table(VIEW_TABLE, schema, tx)?;
        }
        Ok(Self { table_mgr })
    }

    pub fn create_view(&self, name: &str, def: &str, tx: &Arc<Transaction>) -> DbResult<()> {
        let layout = self.table_mgr.get_layout(VIEW_TABLE, tx)?;
        let mut ts = TableScan::new(tx, VIEW_TABLE, layout)?;
        ts.insert()?;
        ts.set_string(&Element::raw(VIEW_NAME), name)?;
        ts.set_string(&Element::raw(VIEW_DEF), def)?;
        Ok(())
    }

    pub fn get_view_def(&self, name: &str, tx: &Arc<Transaction>) -> DbResult<Option<String>> {
        let layout = self.table_mgr.get_layout(VIEW_TABLE, tx)?;
        let mut ts = TableScan::new(tx, VIEW_TABLE, layout)?;
        let mut result = None;
        while ts.next()? {
            if ts.get_string(&Element::raw(VIEW_NAME))? == name {
                result = Some(ts.get_string(&Element::raw(VIEW_DEF))?);
                break;
            }
        }
        ts.close()?;
        Ok(result)
    }
}
