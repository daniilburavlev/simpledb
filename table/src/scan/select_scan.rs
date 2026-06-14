use common::DbResult;

use crate::{predicate::Predicate, scan::Scan};

pub struct SelectScan {
    scan: Box<Scan>,
    predicate: Predicate,
}

impl SelectScan {
    pub fn new(scan: Box<Scan>, predicate: Predicate) -> Self {
        Self { scan, predicate }
    }
}

impl SelectScan {
    pub fn before_first(&self) -> common::DbResult<()> {
        self.scan.before_first()
    }

    pub fn next(&self) -> common::DbResult<bool> {
        while self.scan.next()? {
            if self.predicate.is_satisfied(&self.scan)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn get_i32(&self, field_name: &str) -> common::DbResult<i32> {
        self.scan.get_i32(field_name)
    }

    pub fn get_string(&self, field_name: &str) -> common::DbResult<String> {
        self.scan.get_string(field_name)
    }

    pub fn get_val(&self, field_name: &str) -> common::DbResult<crate::constant::Constant> {
        self.scan.get_val(field_name)
    }

    pub fn has_field(&self, field_name: &str) -> common::DbResult<bool> {
        self.scan.has_field(field_name)
    }

    pub fn close(&self) -> common::DbResult<()> {
        self.scan.close()
    }

    pub fn set_i32(&self, field_name: &str, value: i32) -> DbResult<()> {
        self.scan.set_i32(field_name, value)
    }

    pub fn set_string(&self, field_name: &str, value: &str) -> common::DbResult<()> {
        self.scan.set_string(field_name, value)
    }

    pub fn set_val(
        &self,
        field_name: &str,
        value: crate::constant::Constant,
    ) -> common::DbResult<()> {
        self.scan.set_val(field_name, value)
    }

    pub fn insert(&self) -> common::DbResult<()> {
        self.scan.insert()
    }

    pub fn delete(&self) -> common::DbResult<()> {
        self.scan.delete()
    }

    pub fn get_rid(&self) -> common::DbResult<crate::rid::RID> {
        self.scan.get_rid()
    }

    pub fn move_to_rid(&self, rid: crate::rid::RID) -> common::DbResult<()> {
        self.scan.move_to_rid(rid)
    }
}
