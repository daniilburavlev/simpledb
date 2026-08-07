use std::{rc::Rc, sync::Arc};

use common::{DbResult, error::DbError};
use transaction::transaction::Transaction;

use crate::{
    element::Element,
    metadata_mgr::MetadataMgr,
    plan::{Plan, table::TablePlan},
    query::{
        command::{DeleteData, IndexData, InsertData, QueryData, TableData, UpdateData, ViewData},
        heuristic_planner::check_layout,
        planner::{QueryPlanner, UpdatePlanner},
    },
};

pub struct HeuristicUpdatePlanner {
    query_planner: Rc<dyn QueryPlanner>,
    md: MetadataMgr,
}

impl HeuristicUpdatePlanner {
    pub fn new(query_planner: &Rc<dyn QueryPlanner>, md: MetadataMgr) -> Self {
        Self {
            query_planner: Rc::clone(query_planner),
            md,
        }
    }
}

impl UpdatePlanner for HeuristicUpdatePlanner {
    fn execute_insert(&self, data: InsertData, tx: &Arc<Transaction>) -> DbResult<i32> {
        if data.fields.len() != data.values.len() {
            return Err(DbError::InvalidValuesAmount);
        }
        check_layout(&self.md, &data.table, &data.fields, tx)?;
        let index = self.md.get_index_info(&data.table, tx)?;
        let p = Rc::new(TablePlan::new(tx, data.table.clone(), &self.md)?);
        let s = p.open()?;
        s.insert()?;
        let rid = s.get_rid()?;
        for (field, value) in data.fields.iter().zip(data.values) {
            s.set_val(field, value.clone())?;
            if let Some(index_info) = index.get(field) {
                let index = index_info.open()?;
                index.insert(value, rid)?;
            }
        }
        s.close()?;
        Ok(1)
    }

    fn execute_update(&self, data: UpdateData, tx: &Arc<Transaction>) -> DbResult<i32> {
        let query_data: QueryData = data.clone().into();
        let p = self.query_planner.create_plan(query_data, tx)?;
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

    fn execute_delete(&self, data: DeleteData, tx: &Arc<Transaction>) -> DbResult<i32> {
        let query_data: QueryData = data.clone().into();
        let p = self.query_planner.create_plan(query_data, tx)?;
        let s = p.open()?;
        let mut count = 0;
        while s.next()? {
            s.delete()?;
            count += 1;
        }
        s.close()?;
        Ok(count)
    }

    fn execute_create_table(&self, data: TableData, tx: &Arc<Transaction>) -> DbResult<i32> {
        self.md.create_table(&data.name, data.schema, tx)?;
        Ok(0)
    }

    fn execute_create_view(&self, data: ViewData, tx: &Arc<Transaction>) -> DbResult<i32> {
        self.md
            .create_view(&data.name, &data.query.to_string(), tx)?;
        Ok(0)
    }

    fn execute_create_index(&self, data: IndexData, tx: &Arc<Transaction>) -> DbResult<i32> {
        let field = Element::raw(&data.field);
        check_layout(&self.md, &data.table, std::slice::from_ref(&field), tx)?;
        self.md
            .create_index(&data.index, &data.table, &data.field, tx)?;
        let table = Element::raw(&data.table);
        let mut query_data = QueryData::new(table);
        query_data.fields = vec![field.clone()];
        let p = self.query_planner.create_plan(query_data, tx)?;
        let s = p.open()?;
        let indexes = self.md.get_index_info(&data.table, tx)?;
        let Some(index) = indexes.get(&field) else {
            return Ok(0);
        };
        let index = index.open()?;
        while s.next()? {
            let value = s.get_val(&field)?;
            let rid = s.get_rid()?;
            index.insert(value, rid)?;
        }
        Ok(0)
    }
}
