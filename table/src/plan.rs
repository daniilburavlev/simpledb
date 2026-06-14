use std::sync::Arc;

use common::DbResult;

use crate::{
    plan::{product::ProductPlan, project::ProjectPlan, select::SelectPlan, table::TablePlan},
    scan::Scan,
    schema::Schema,
};

pub mod product;
pub mod project;
pub mod select;
pub mod table;

pub enum Plan {
    Table(TablePlan),
    Select(SelectPlan),
    Product(ProductPlan),
    Project(ProjectPlan),
}

impl Plan {
    pub fn open(&self) -> DbResult<Scan> {
        match self {
            Self::Table(table) => Ok(Scan::Table(table.open()?)),
            Self::Select(select) => Ok(Scan::Select(select.open()?)),
            Self::Product(select) => Ok(Scan::Product(select.open()?)),
            Self::Project(select) => Ok(Scan::Project(select.open()?)),
        }
    }

    pub fn blocks_accessed(&self) -> DbResult<i32> {
        match self {
            Self::Table(table) => table.blocks_accessed(),
            Self::Select(select) => select.blocks_accessed(),
            Self::Product(select) => select.blocks_accessed(),
            Self::Project(select) => select.blocks_accessed(),
        }
    }

    pub fn records_output(&self) -> DbResult<i32> {
        match self {
            Self::Table(table) => table.records_output(),
            Self::Select(select) => select.records_output(),
            Self::Product(select) => select.records_output(),
            Self::Project(select) => select.records_output(),
        }
    }

    pub fn distinct_values(&self, field_name: &str) -> DbResult<i32> {
        match self {
            Self::Table(table) => table.distinct_values(field_name),
            Self::Select(select) => select.distinct_values(field_name),
            Self::Product(select) => select.distinct_values(field_name),
            Self::Project(select) => select.distinct_values(field_name),
        }
    }

    pub fn schema(&self) -> DbResult<Arc<Schema>> {
        match self {
            Self::Table(table) => table.schema(),
            Self::Select(select) => select.schema(),
            Self::Product(select) => select.schema(),
            Self::Project(select) => select.schema(),
        }
    }
}
