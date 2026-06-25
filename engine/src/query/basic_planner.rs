use std::{rc::Rc, sync::Arc};

use common::{DbResult, error::DbError};
use transaction::transaction::Transaction;

use crate::{
    metadata_mgr::MetadataMgr,
    plan::{Plan, select::SelectPlan, table::TablePlan},
    query::{command::IndexData, planner::UpdatePlanner},
};

pub struct BasicUpdatePlanner {
    md: Arc<MetadataMgr>,
}

impl BasicUpdatePlanner {
    pub fn new(md: &Arc<MetadataMgr>) -> Self {
        Self { md: Arc::clone(md) }
    }
}

impl UpdatePlanner for BasicUpdatePlanner {
    fn execute_insert(
        &self,
        data: super::command::InsertData,
        tx: &Arc<Transaction>,
    ) -> DbResult<i32> {
        let p = Rc::new(TablePlan::new(tx, data.table.clone(), &self.md)?);
        let s = p.open()?;
        s.insert()?;
        if data.fields.len() != data.values.len() {
            return Err(DbError::InvalidValuesAmount);
        }
        let mut count = 0;
        for (field, value) in data.fields.iter().zip(data.values) {
            s.set_val(field, value)?;
            count += 1;
        }
        s.close()?;
        Ok(count)
    }

    fn execute_update(
        &self,
        data: super::command::UpdateData,
        tx: &Arc<Transaction>,
    ) -> DbResult<i32> {
        let p = Rc::new(TablePlan::new(tx, data.table, &self.md)?);
        let p = Rc::new(SelectPlan::new(p, data.predicate));
        let s = p.open()?;
        let mut count = 0;
        while s.next()? {
            let val = data.value.evaluate(&s)?;
            s.set_val(&data.field, val)?;
            count += 1;
        }
        s.close()?;
        Ok(count)
    }

    fn execute_delete(
        &self,
        data: super::command::DeleteData,
        tx: &Arc<Transaction>,
    ) -> DbResult<i32> {
        let p = Rc::new(TablePlan::new(tx, data.name, &self.md)?);
        let p = Rc::new(SelectPlan::new(p, data.predicate));
        let s = p.open()?;
        let mut count = 0;
        while s.next()? {
            s.delete()?;
            count += 1;
        }
        s.close()?;
        Ok(count)
    }

    fn execute_create_table(
        &self,
        data: super::command::TableData,
        tx: &Arc<Transaction>,
    ) -> DbResult<i32> {
        self.md
            .create_table(&data.name, &Arc::new(data.schema), tx)?;
        Ok(0)
    }

    fn execute_create_view(
        &self,
        data: super::command::ViewData,
        tx: &Arc<Transaction>,
    ) -> DbResult<i32> {
        self.md
            .create_view(&data.name, &data.query.to_string(), tx)?;
        Ok(0)
    }

    fn execute_create_index(&self, data: IndexData, tx: &Arc<Transaction>) -> DbResult<i32> {
        self.md
            .create_index(&data.index, &data.table, &data.field, tx)?;
        Ok(0)
    }
}
