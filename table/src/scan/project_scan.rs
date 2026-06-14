use std::collections::HashSet;

use common::error::DbError;

use crate::scan::Scan;

pub struct ProjectScan {
    scan: Box<Scan>,
    fields: HashSet<String>,
}

impl ProjectScan {
    pub fn new(scan: Box<Scan>, fields: HashSet<String>) -> Self {
        Self { scan, fields }
    }
}

impl ProjectScan {
    pub fn before_first(&self) -> common::DbResult<()> {
        self.scan.before_first()
    }

    pub fn next(&self) -> common::DbResult<bool> {
        self.scan.next()
    }

    pub fn get_i32(&self, field_name: &str) -> common::DbResult<i32> {
        if self.has_field(field_name)? {
            self.scan.get_i32(field_name)
        } else {
            Err(DbError::field_not_exists(field_name))
        }
    }

    pub fn get_string(&self, field_name: &str) -> common::DbResult<String> {
        if self.has_field(field_name)? {
            self.scan.get_string(field_name)
        } else {
            Err(DbError::field_not_exists(field_name))
        }
    }

    pub fn get_val(&self, field_name: &str) -> common::DbResult<crate::constant::Constant> {
        if self.has_field(field_name)? {
            self.scan.get_val(field_name)
        } else {
            Err(DbError::field_not_exists(field_name))
        }
    }

    pub fn has_field(&self, field_name: &str) -> common::DbResult<bool> {
        Ok(self.fields.contains(field_name))
    }

    pub fn close(&self) -> common::DbResult<()> {
        self.scan.close()
    }
}
