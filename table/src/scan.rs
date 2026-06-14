use common::{DbResult, error::DbError};

use crate::{
    constant::Constant,
    rid::RID,
    scan::{
        product_scan::ProductScan, project_scan::ProjectScan, select_scan::SelectScan,
        table_scan::TableScan,
    },
};

pub mod product_scan;
pub mod project_scan;
pub mod select_scan;
pub mod table_scan;

pub enum Scan {
    Table(TableScan),
    Select(SelectScan),
    Product(ProductScan),
    Project(ProjectScan),
}

impl Scan {
    pub fn before_first(&self) -> DbResult<()> {
        match self {
            Self::Table(table) => table.before_first(),
            Self::Select(select) => select.before_first(),
            Self::Product(product) => product.before_first(),
            Self::Project(project) => project.before_first(),
        }
    }

    pub fn next(&self) -> DbResult<bool> {
        match self {
            Self::Table(table) => table.next(),
            Self::Select(select) => select.next(),
            Self::Product(product) => product.next(),
            Self::Project(project) => project.next(),
        }
    }

    pub fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        match self {
            Self::Table(table) => table.get_i32(field_name),
            Self::Select(select) => select.get_i32(field_name),
            Self::Product(product) => product.get_i32(field_name),
            Self::Project(project) => project.get_i32(field_name),
        }
    }

    pub fn get_string(&self, field_name: &str) -> DbResult<String> {
        match self {
            Self::Table(table) => table.get_string(field_name),
            Self::Select(select) => select.get_string(field_name),
            Self::Product(table) => table.get_string(field_name),
            Self::Project(table) => table.get_string(field_name),
        }
    }

    pub fn get_val(&self, field_name: &str) -> DbResult<Constant> {
        match self {
            Self::Table(table) => table.get_val(field_name),
            Self::Select(select) => select.get_val(field_name),
            Self::Product(table) => table.get_val(field_name),
            Self::Project(table) => table.get_val(field_name),
        }
    }

    pub fn has_field(&self, field_name: &str) -> DbResult<bool> {
        match self {
            Self::Table(table) => table.has_field(field_name),
            Self::Select(select) => select.has_field(field_name),
            Self::Product(table) => table.has_field(field_name),
            Self::Project(table) => table.has_field(field_name),
        }
    }

    pub fn close(&self) -> DbResult<()> {
        match self {
            Self::Table(table) => table.close(),
            Self::Select(select) => select.close(),
            Self::Product(table) => table.close(),
            Self::Project(table) => table.close(),
        }
    }

    pub fn set_i32(&self, field_name: &str, value: i32) -> DbResult<()> {
        match self {
            Self::Table(table) => table.set_i32(field_name, value),
            Self::Select(select) => select.set_i32(field_name, value),
            _ => Err(DbError::BadSyntax),
        }
    }

    pub fn set_string(&self, field_name: &str, value: &str) -> DbResult<()> {
        match self {
            Self::Table(table) => table.set_string(field_name, value),
            Self::Select(select) => select.set_string(field_name, value),
            _ => Err(DbError::BadSyntax),
        }
    }

    pub fn set_val(&self, field_name: &str, value: Constant) -> DbResult<()> {
        match self {
            Self::Table(table) => table.set_val(field_name, value),
            Self::Select(select) => select.set_val(field_name, value),
            _ => Err(DbError::BadSyntax),
        }
    }

    pub fn insert(&self) -> DbResult<()> {
        match self {
            Self::Table(table) => table.insert(),
            Self::Select(select) => select.insert(),
            _ => Err(DbError::BadSyntax),
        }
    }

    pub fn delete(&self) -> DbResult<()> {
        match self {
            Self::Table(table) => table.delete(),
            Self::Select(select) => select.delete(),
            _ => Err(DbError::BadSyntax),
        }
    }

    pub fn get_rid(&self) -> DbResult<RID> {
        match self {
            Self::Table(table) => table.get_rid(),
            Self::Select(select) => select.get_rid(),
            _ => Err(DbError::BadSyntax),
        }
    }

    pub fn move_to_rid(&self, rid: RID) -> DbResult<()> {
        match self {
            Self::Table(table) => table.move_to_rid(rid),
            Self::Select(select) => select.move_to_rid(rid),
            _ => Err(DbError::BadSyntax),
        }
    }
}
