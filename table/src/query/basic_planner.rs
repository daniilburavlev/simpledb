use std::{rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    metadata_mgr::MetadataMgr,
    plan::{Plan, product::ProductPlan, project::ProjectPlan, table::TablePlan},
    query::{
        command::{Command, QueryData},
        parser::Parser,
        planner::QueryPlanner,
    },
};

pub struct BasicQueryPlanner {
    md: Arc<MetadataMgr>,
}

impl BasicQueryPlanner {
    pub fn new(md: &Arc<MetadataMgr>) -> Self {
        Self { md: Arc::clone(md) }
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
                plans.push(Rc::new(TablePlan::new(tx, table, &self.md)?));
            }
        }
        let mut p = plans.remove(0);
        for next in plans.into_iter().skip(1) {
            p = Rc::new(ProductPlan::new(p, next)?);
        }
        Ok(Rc::new(ProjectPlan::new(p, data.fields)?))
    }
}

pub struct BasicUpdatePlanner {}
