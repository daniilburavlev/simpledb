use common::{DbResult, error::DbError};

use crate::{
    element::Element, rid::RID, schema::Schema, select::SelectScan, table::TableScan, value::Value,
};

pub(crate) enum Scanner {
    Table(TableScan),
    Select(SelectScan),
}

impl Scanner {
    pub(crate) fn before_first(&mut self) -> DbResult<()> {
        match self {
            Self::Table(table) => table.before_first(),
            Self::Select(select) => select.before_first(),
        }
    }

    pub fn next(&mut self) -> DbResult<bool> {
        match self {
            Self::Table(table) => table.next(),
            Self::Select(select) => select.next(),
        }
    }

    pub fn get_i32(&self, field: &Element) -> DbResult<i32> {
        match self {
            Self::Table(table) => table.get_i32(field),
            Self::Select(select) => select.get_i32(field),
        }
    }

    pub fn get_string(&self, field: &Element) -> DbResult<String> {
        match self {
            Self::Table(table) => table.get_string(field),
            Self::Select(select) => select.get_string(field),
        }
    }

    pub fn get_val(&self, field: &Element) -> DbResult<Value> {
        match self {
            Self::Table(table) => table.get_val(field),
            Self::Select(select) => select.get_val(field),
        }
    }

    pub fn has_field(&self, field: &Element) -> DbResult<bool> {
        match self {
            Self::Table(table) => Ok(table.has_field(field)),
            Self::Select(select) => select.has_field(field),
        }
    }

    pub fn close(&self) -> DbResult<()> {
        match self {
            Self::Table(table) => table.close(),
            Self::Select(select) => select.close(),
        }
    }

    pub fn schema(&self) -> DbResult<Schema> {
        match self {
            Self::Table(table) => Ok(table.schema().clone()),
            Self::Select(select) => select.schema(),
        }
    }

    pub fn set_i32(&self, field: &Element, value: i32) -> DbResult<()> {
        match self {
            Self::Table(table) => table.set_i32(field, value),
            Self::Select(select) => select.set_i32(field, value),
        }
    }

    pub fn set_string(&self, field: &Element, value: &str) -> DbResult<()> {
        match self {
            Self::Table(table) => table.set_string(field, value),
            Self::Select(select) => select.set_string(field, value),
        }
    }

    pub fn set_val(&self, field: &Element, value: Value) -> DbResult<()> {
        match self {
            Self::Table(table) => table.set_val(field, value),
            Self::Select(select) => select.set_val(field, value),
        }
    }

    pub fn insert(&mut self) -> DbResult<()> {
        match self {
            Self::Table(table) => table.insert(),
            Self::Select(select) => select.insert(),
        }
    }

    pub fn delete(&self) -> DbResult<()> {
        match self {
            Self::Table(table) => table.delete(),
            Self::Select(select) => select.delete(),
        }
    }

    pub fn get_rid(&self) -> DbResult<RID> {
        match self {
            Self::Table(table) => Ok(table.get_rid()),
            Self::Select(select) => select.get_rid(),
        }
    }

    pub fn move_to_rid(&mut self, rid: RID) -> DbResult<()> {
        match self {
            Self::Table(table) => table.move_to_rid(rid),
            Self::Select(table) => table.move_to_rid(rid),
        }
    }

    pub fn save_position(&self) -> DbResult<()> {
        Err(DbError::other("cannot save position"))
    }
}
