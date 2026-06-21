use std::sync::Arc;

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    scan::{Scan, table::TableScan},
    schema::Schema,
    table_mgr::TableMgr,
};

const MAX_NAME: i32 = 16;
const MAX_VIEW_DEF: i32 = 100;

const VIEW_NAME: &str = "view";
const VIEW_DEF: &str = "view_def";
const VIEW_TABLE: &str = "sp_view";

pub struct ViewMgr {
    table_mgr: Arc<TableMgr>,
}

impl ViewMgr {
    pub fn new(is_new: bool, table_mgr: &Arc<TableMgr>, tx: &Arc<Transaction>) -> DbResult<Self> {
        if is_new {
            let schema = Arc::new(Schema::default());
            schema.add_string_field(VIEW_NAME.to_string(), MAX_NAME)?;
            schema.add_string_field(VIEW_DEF.to_string(), MAX_VIEW_DEF)?;
            table_mgr.create_table(VIEW_TABLE, &schema, tx)?;
        }
        Ok(Self {
            table_mgr: Arc::clone(table_mgr),
        })
    }

    pub fn create_view(&self, name: &str, def: &str, tx: &Arc<Transaction>) -> DbResult<()> {
        let layout = Arc::new(self.table_mgr.get_layout(VIEW_TABLE, tx)?);
        let ts = TableScan::new(tx, VIEW_TABLE, &layout)?;
        ts.insert()?;
        ts.set_string(VIEW_NAME, name)?;
        ts.set_string(VIEW_DEF, def)?;
        Ok(())
    }

    pub fn get_view_def(&self, name: &str, tx: &Arc<Transaction>) -> DbResult<Option<String>> {
        let layout = self.table_mgr.get_layout(VIEW_TABLE, tx)?;
        let ts = TableScan::new(tx, VIEW_TABLE, &Arc::new(layout))?;
        let mut result = None;
        while ts.next()? {
            if ts.get_string(VIEW_NAME)? == name {
                result = Some(ts.get_string(VIEW_DEF)?);
                break;
            }
        }
        ts.close()?;
        Ok(result)
    }
}
