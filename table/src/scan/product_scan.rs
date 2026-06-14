use common::DbResult;

use crate::scan::Scan;

pub struct ProductScan {
    s1: Box<Scan>,
    s2: Box<Scan>,
}

impl ProductScan {
    pub fn new(s1: Box<Scan>, s2: Box<Scan>) -> DbResult<Self> {
        s1.next()?;
        Ok(Self { s1, s2 })
    }
}

impl ProductScan {
    pub fn before_first(&self) -> common::DbResult<()> {
        self.s1.before_first()?;
        self.s1.next()?;
        self.s2.before_first()
    }

    pub fn next(&self) -> common::DbResult<bool> {
        if self.s2.next()? {
            Ok(true)
        } else {
            self.s2.before_first()?;
            Ok(self.s1.next()? && self.s2.next()?)
        }
    }

    pub fn get_i32(&self, field_name: &str) -> common::DbResult<i32> {
        if self.s1.has_field(field_name)? {
            self.s1.get_i32(field_name)
        } else {
            self.s2.get_i32(field_name)
        }
    }

    pub fn get_string(&self, field_name: &str) -> common::DbResult<String> {
        if self.s1.has_field(field_name)? {
            self.s1.get_string(field_name)
        } else {
            self.s2.get_string(field_name)
        }
    }

    pub fn get_val(&self, field_name: &str) -> common::DbResult<crate::constant::Constant> {
        if self.s1.has_field(field_name)? {
            self.s1.get_val(field_name)
        } else {
            self.s2.get_val(field_name)
        }
    }

    pub fn has_field(&self, field_name: &str) -> common::DbResult<bool> {
        Ok(self.s1.has_field(field_name)? || self.s2.has_field(field_name)?)
    }

    pub fn close(&self) -> common::DbResult<()> {
        self.s1.close()?;
        self.s2.close()
    }
}
