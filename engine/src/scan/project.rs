use crate::element::Element;
use crate::scan::Scan;
use crate::schema::{Schema, SchemaBuilder};
use common::DbResult;
use common::error::DbError;
use std::sync::Arc;
use std::{collections::HashSet, rc::Rc};

pub struct ProjectScan {
    scan: Rc<dyn Scan>,
    fields: HashSet<Element>,
}

impl ProjectScan {
    pub fn new(scan: Rc<dyn Scan>, fields: HashSet<Element>) -> Self {
        Self { scan, fields }
    }
}

impl Scan for ProjectScan {
    fn before_first(&self) -> DbResult<()> {
        self.scan.before_first()
    }

    fn next(&self) -> DbResult<bool> {
        self.scan.next()
    }

    fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        if self.has_field(field_name)? {
            self.scan.get_i32(field_name)
        } else {
            Err(DbError::FieldNotExists(field_name.to_string()))
        }
    }

    fn get_string(&self, field_name: &Element) -> DbResult<String> {
        if self.has_field(field_name)? {
            self.scan.get_string(field_name)
        } else {
            Err(DbError::FieldNotExists(field_name.to_string()))
        }
    }

    fn get_val(&self, field_name: &Element) -> DbResult<crate::value::Value> {
        if self.has_field(field_name)? {
            self.scan.get_val(field_name)
        } else {
            Err(DbError::FieldNotExists(field_name.to_string()))
        }
    }

    fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        Ok(self.fields.contains(field_name))
    }

    fn close(&self) -> DbResult<()> {
        self.scan.close()
    }

    fn schema(&self) -> DbResult<Schema> {
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
