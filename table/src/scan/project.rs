use std::{collections::HashSet, rc::Rc};

use common::error::DbError;

use crate::scan::Scan;

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
    fn before_first(&self) -> common::DbResult<()> {
        self.scan.before_first()
    }

    fn next(&self) -> common::DbResult<bool> {
        self.scan.next()
    }

    fn get_i32(&self, field_name: &str) -> common::DbResult<i32> {
        if self.has_field(field_name)? {
            self.scan.get_i32(field_name)
        } else {
            Err(DbError::field_not_exists(field_name))
        }
    }

    fn get_string(&self, field_name: &str) -> common::DbResult<String> {
        if self.has_field(field_name)? {
            self.scan.get_string(field_name)
        } else {
            Err(DbError::field_not_exists(field_name))
        }
    }

    fn get_val(&self, field_name: &str) -> common::DbResult<crate::constant::Constant> {
        if self.has_field(field_name)? {
            self.scan.get_val(field_name)
        } else {
            Err(DbError::field_not_exists(field_name))
        }
    }

    fn has_field(&self, field_name: &str) -> common::DbResult<bool> {
        Ok(self.fields.contains(field_name))
    }

    fn close(&self) -> common::DbResult<()> {
        self.scan.close()
    }
}
