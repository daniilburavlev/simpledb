use crate::scan::Scan;
use crate::schema::Schema;
use common::DbResult;
use common::error::DbError;
use std::sync::Arc;
use std::{collections::HashSet, rc::Rc};

pub struct ProjectScan {
    scan: Rc<dyn Scan>,
    fields: HashSet<String>,
}

impl ProjectScan {
    pub fn new(scan: Rc<dyn Scan>, fields: HashSet<String>) -> Self {
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

    fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        if self.has_field(field_name)? {
            self.scan.get_i32(field_name)
        } else {
            Err(DbError::field_not_exists(field_name))
        }
    }

    fn get_string(&self, field_name: &str) -> DbResult<String> {
        if self.has_field(field_name)? {
            self.scan.get_string(field_name)
        } else {
            Err(DbError::field_not_exists(field_name))
        }
    }

    fn get_val(&self, field_name: &str) -> DbResult<crate::constant::Constant> {
        if self.has_field(field_name)? {
            self.scan.get_val(field_name)
        } else {
            Err(DbError::field_not_exists(field_name))
        }
    }

    fn has_field(&self, field_name: &str) -> DbResult<bool> {
        Ok(self.fields.contains(field_name))
    }

    fn close(&self) -> DbResult<()> {
        self.scan.close()
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        let project = Arc::new(Schema::default());
        let schema = self.scan.schema()?;
        for (field, info) in schema.fields()? {
            if self.fields.contains(&field) {
                project.add_field(field, info)?;
            }
        }
        Ok(project)
    }
}
