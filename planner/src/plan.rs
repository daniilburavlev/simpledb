use std::sync::Arc;

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    element::Element,
    mgr::metadata::MetadataMgr,
    plan::{select::SelectPlan, table::TablePlan},
    predicate::Predicate,
    scan::Scan,
    schema::Schema,
};

pub(crate) mod select;
pub(crate) mod table;
mod project;

pub(crate) enum Planner {
    Table(TablePlan),
    Select(SelectPlan),
}

pub struct Plan(Planner);

impl Plan {
    pub fn table(tx: &Arc<Transaction>, table: String, md: &MetadataMgr) -> DbResult<Self> {
        Ok(Self(Planner::Table(TablePlan::new(tx, table, md)?)))
    }

    pub fn select(plan: Box<Plan>, predicate: Predicate) -> Self {
        Self(Planner::Select(SelectPlan::new(plan, predicate)))
    }

    pub fn project(plan: Box<Plan>, predicate: Predicate) -> Self {
        Self(Planner::Select(SelectPlan::new(plan, predicate)))
    }

    pub fn open(&self) -> DbResult<Scan> {
        match &self.0 {
            Planner::Table(table) => table.open(),
            Planner::Select(table) => table.open(),
        }
    }

    pub fn blocks_accessed(&self) -> DbResult<i32> {
        match &self.0 {
            Planner::Table(table) => table.blocks_accessed(),
            Planner::Select(table) => table.blocks_accessed(),
        }
    }

    pub fn records_output(&self) -> DbResult<i32> {
        match &self.0 {
            Planner::Table(table) => table.records_output(),
            Planner::Select(table) => table.records_output(),
        }
    }

    pub fn distinct_values(&self, field_name: &Element) -> DbResult<i32> {
        match &self.0 {
            Planner::Table(table) => table.distinct_values(field_name),
            Planner::Select(table) => table.distinct_values(field_name),
        }
    }

    pub fn schema(&self) -> DbResult<Schema> {
        match &self.0 {
            Planner::Table(table) => Ok(table.schema()),
            Planner::Select(table) => table.schema(),
        }
    }
}
