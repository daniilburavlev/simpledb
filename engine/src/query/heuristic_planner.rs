use std::{rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    metadata_mgr::MetadataMgr,
    plan::{Plan, project::ProjectPlan},
    query::{command::QueryData, table_planner::TablePlanner},
};

pub(crate) struct HeuristicQueryPlanner {
    table_planners: Vec<TablePlanner>,
    md: Arc<MetadataMgr>,
}

impl HeuristicQueryPlanner {
    pub(crate) fn new(md: &Arc<MetadataMgr>) -> Self {
        Self {
            table_planners: vec![],
            md: Arc::clone(md),
        }
    }

    pub(crate) fn create_plan(
        &mut self,
        data: QueryData,
        tx: &Arc<Transaction>,
    ) -> DbResult<Rc<dyn Plan>> {
        for table in &data.tables {
            let tp = TablePlanner::new(table, data.predicate.clone(), tx, &self.md)?;
            self.table_planners.push(tp);
        }
        let mut current = self.get_lowest_select_plan()?;
        while !self.table_planners.is_empty() {
            if let Some(p) = self.get_lowest_join_plan(&current)? {
                current = p;
            } else {
                current = self.get_lowest_product_plan(&current)?;
            }
        }
        Ok(Rc::new(ProjectPlan::new(current, data.fields)?))
    }

    fn get_lowest_select_plan(&mut self) -> DbResult<Rc<dyn Plan>> {
        let mut index = 0;
        let mut best_plan = self.table_planners.first().unwrap().make_select_plan()?;
        for (i, tp) in self.table_planners.iter().skip(1).enumerate() {
            let plan = tp.make_select_plan()?;
            if plan.records_output()? < best_plan.records_output()? {
                index = i;
                best_plan = plan;
            }
        }
        self.table_planners.remove(index);
        Ok(best_plan)
    }

    fn get_lowest_join_plan(&mut self, current: &Rc<dyn Plan>) -> DbResult<Option<Rc<dyn Plan>>> {
        let mut index = 0;
        let mut best_plan = None;
        for (i, tp) in self.table_planners.iter().enumerate() {
            let Some(plan) = tp.make_join_plan(current)? else {
                continue;
            };
            let Some(p) = &best_plan else {
                index = i;
                best_plan = Some(plan);
                continue;
            };
            if plan.records_output()? < p.records_output()? {
                index = i;
                best_plan = Some(plan);
            }
        }
        if best_plan.is_some() {
            self.table_planners.remove(index);
        }
        Ok(best_plan)
    }

    fn get_lowest_product_plan(&mut self, current: &Rc<dyn Plan>) -> DbResult<Rc<dyn Plan>> {
        let mut index = 0;
        let mut best_plan = self
            .table_planners
            .first()
            .unwrap()
            .make_product_plan(current)?;
        for (i, tp) in self.table_planners.iter().skip(1).enumerate() {
            let plan = tp.make_product_plan(current)?;
            if plan.records_output()? < best_plan.records_output()? {
                index = 1;
                best_plan = plan;
            }
        }
        self.table_planners.remove(index);
        Ok(best_plan)
    }
}
