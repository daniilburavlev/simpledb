use std::{rc::Rc, sync::Arc};

use common::{DbResult, error::DbError};
use transaction::transaction::Transaction;

use crate::{
    metadata_mgr::MetadataMgr,
    plan::{
        Plan, group::GroupByPlan, index::IndexSelectPlan, product::ProductPlan,
        project::ProjectPlan, select::SelectPlan, sort::SortPlan, table::TablePlan,
    },
    predicate::Predicate,
    query::{
        command::{Command, IndexData, QueryData},
        parser::Parser,
        planner::{QueryPlanner, UpdatePlanner},
    },
};

pub struct BasicQueryPlanner {
    md: Arc<MetadataMgr>,
}

impl BasicQueryPlanner {
    pub fn new(md: &Arc<MetadataMgr>) -> Self {
        Self { md: Arc::clone(md) }
    }

    /// Build a scan plan for a single table, preferring an index select plan
    /// when the predicate equates an indexed field with a constant.
    fn table_plan(
        &self,
        table: String,
        predicate: &Predicate,
        tx: &Arc<Transaction>,
    ) -> DbResult<Rc<dyn Plan>> {
        let base: Rc<dyn Plan> = Rc::new(TablePlan::new(tx, table.clone(), &self.md)?);
        let indexes = self.md.get_index_info(&table, tx)?;

        for (field, info) in indexes {
            if let Some(value) = predicate.equates_with_constant(&field)? {
                tracing::debug!("Using index on {}.{}", table, field);
                return Ok(Rc::new(IndexSelectPlan::new(&base, info, value)));
            }
        }
        Ok(base)
    }
}

impl QueryPlanner for BasicQueryPlanner {
    fn create_plan(&self, data: QueryData, tx: &Arc<Transaction>) -> DbResult<Rc<dyn Plan>> {
        let mut plans = vec![];
        for table in data.tables {
            if let Some(view) = self.md.get_view_def(&table, tx)? {
                let parser = Parser::new(&view)?;
                if let Command::Query(data) = parser.query()? {
                    plans.push(self.create_plan(data, tx)?);
                }
            } else {
                plans.push(self.table_plan(table, &data.predicate, tx)?);
            }
        }
        let mut p = plans.remove(0);
        for next in plans.into_iter().skip(1) {
            let p1 = ProductPlan::new(next.clone(), p.clone())?;
            let p2 = ProductPlan::new(p.clone(), next.clone())?;
            p = if p1.blocks_accessed()? < p2.blocks_accessed()? {
                Rc::new(p1)
            } else {
                Rc::new(p2)
            };
        }
        let mut p: Rc<dyn Plan> = Rc::new(SelectPlan::new(p, data.predicate));
        if !data.group_by.is_empty() {
            p = Rc::new(GroupByPlan::new(tx, &p, data.group_by.fields, vec![])?);
        }
        if !data.sort_by.is_empty() {
            p = Rc::new(SortPlan::new(tx, &p, data.sort_by.fields)?);
        }
        Ok(Rc::new(ProjectPlan::new(p, data.fields)?))
    }
}

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
