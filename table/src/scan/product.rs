use common::DbResult;

use crate::scan::Scan;

pub struct ProductScan {
    s1: Box<dyn Scan>,
    s2: Box<dyn Scan>,
}

impl ProductScan {
    pub fn new(s1: Box<dyn Scan>, s2: Box<dyn Scan>) -> DbResult<Self> {
        s1.next()?;
        Ok(Self { s1, s2 })
    }
}

impl Scan for ProductScan {
    fn before_first(&self) -> common::DbResult<()> {
        self.s1.before_first()?;
        self.s1.next()?;
        self.s2.before_first()
    }

    fn next(&self) -> common::DbResult<bool> {
        if self.s2.next()? {
            Ok(true)
        } else {
            self.s2.before_first()?;
            Ok(self.s1.next()? && self.s2.next()?)
        }
    }

    fn get_i32(&self, field_name: &str) -> common::DbResult<i32> {
        if self.s1.has_field(field_name)? {
            self.s1.get_i32(field_name)
        } else {
            self.s2.get_i32(field_name)
        }
    }

    fn get_string(&self, field_name: &str) -> common::DbResult<String> {
        if self.s1.has_field(field_name)? {
            self.s1.get_string(field_name)
        } else {
            self.s2.get_string(field_name)
        }
    }

    fn get_val(&self, field_name: &str) -> common::DbResult<crate::constant::Constant> {
        if self.s1.has_field(field_name)? {
            self.s1.get_val(field_name)
        } else {
            self.s2.get_val(field_name)
        }
    }

    fn has_field(&self, field_name: &str) -> common::DbResult<bool> {
        Ok(self.s1.has_field(field_name)? || self.s2.has_field(field_name)?)
    }

    fn close(&self) -> common::DbResult<()> {
        self.s1.close()?;
        self.s2.close()
    }
}
