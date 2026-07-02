use std::{cell::RefCell, rc::Rc, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::element::Element;
use crate::plan::group::GroupByPlan;
use crate::plan::order::SortPlan;
use crate::{
    metadata_mgr::MetadataMgr,
    plan::{Plan, project::ProjectPlan},
    query::{command::QueryData, planner::QueryPlanner, table_planner::TablePlanner},
};

struct HeuristicQueryPlannerInner {
    table_planners: Vec<TablePlanner>,
    md: MetadataMgr,
}

impl HeuristicQueryPlannerInner {
    fn new(md: MetadataMgr) -> Self {
        Self {
            table_planners: vec![],
            md,
        }
    }

    fn create_plan(&mut self, data: QueryData, tx: &Arc<Transaction>) -> DbResult<Rc<dyn Plan>> {
        let tables = match &data.table {
            Element::Array(tables) => tables.iter().map(|t| t.as_ref().clone()).collect(),
            table => vec![table.clone()],
        };
        for table in tables {
            let table = if let Some(source) = data.mapping.table(&table)
                && *source != table
            {
                source.clone()
            } else {
                table
            };
            let tp = TablePlanner::new(
                table,
                data.predicate.clone(),
                tx,
                &self.md,
                data.mapping.clone(),
            )?;
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
        if !data.group_by.is_empty() {
            current = Rc::new(GroupByPlan::new(
                tx,
                &current,
                data.group_by.fields,
                vec![],
            )?);
        }
        if !data.order_by.is_empty() {
            current = Rc::new(SortPlan::new(tx, &current, data.order_by.fields)?);
        }
        Ok(Rc::new(ProjectPlan::new(
            current,
            data.fields,
            data.mapping,
        )?))
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
        for tp in self.table_planners.iter().skip(1) {
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

pub(crate) struct HeuristicQueryPlanner(RefCell<HeuristicQueryPlannerInner>);

impl HeuristicQueryPlanner {
    pub(crate) fn new(md: MetadataMgr) -> Self {
        Self(RefCell::new(HeuristicQueryPlannerInner::new(md)))
    }
}

impl QueryPlanner for HeuristicQueryPlanner {
    fn create_plan(&self, data: QueryData, tx: &Arc<Transaction>) -> DbResult<Rc<dyn Plan>> {
        let mut write = self.0.borrow_mut();
        write.create_plan(data, tx)
    }
}
