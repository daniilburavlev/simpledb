use crate::element::Element;
use crate::scan::Scan;
use crate::schema::{Schema, SchemaBuilder};
use crate::value::Value;
use common::DbResult;
use common::error::DbError;
use std::collections::HashSet;

pub struct ProjectScan {
    scan: Box<Scan>,
    fields: HashSet<Element>,
}

impl ProjectScan {
    pub fn new(scan: Box<Scan>, fields: HashSet<Element>) -> Self {
        Self { scan, fields }
    }
}

impl ProjectScan {
    pub(crate) fn before_first(&mut self) -> DbResult<()> {
        self.scan.before_first()
    }

    pub(crate) fn next_row(&mut self) -> DbResult<bool> {
        self.scan.next_row()
    }

    pub(crate) fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        if self.has_field(field_name)? {
            self.scan.get_i32(field_name)
        } else {
            Err(DbError::FieldNotExists(field_name.to_string()))
        }
    }

    pub(crate) fn get_string(&self, field_name: &Element) -> DbResult<String> {
        if self.has_field(field_name)? {
            self.scan.get_string(field_name)
        } else {
            Err(DbError::FieldNotExists(field_name.to_string()))
        }
    }

    pub(crate) fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        if self.has_field(field_name)? {
            self.scan.get_val(field_name)
        } else {
            Err(DbError::FieldNotExists(field_name.to_string()))
        }
    }

    pub(crate) fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        Ok(self.fields.contains(field_name))
    }

    pub(crate) fn close(&self) -> DbResult<()> {
        self.scan.close()
    }

    pub(crate) fn schema(&self) -> DbResult<Schema> {
        let mut project = SchemaBuilder::default();
        let schema = self.scan.schema()?;
        for (field, info) in schema.fields() {
            if self.fields.contains(&field) {
                project = project.add_field(field, info);
            }
        }
        Ok(project.build())
    }
}
