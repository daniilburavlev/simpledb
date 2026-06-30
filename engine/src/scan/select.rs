use common::DbResult;
use std::rc::Rc;

use crate::schema::Schema;
use crate::{predicate::Predicate, scan::Scan};
use crate::element::Element;

pub struct SelectScan {
    scan: Rc<dyn Scan>,
    predicate: Predicate,
}

impl SelectScan {
    pub fn new(scan: Rc<dyn Scan>, predicate: Predicate) -> Self {
        Self { scan, predicate }
    }
}

impl Scan for SelectScan {
    fn before_first(&self) -> DbResult<()> {
        self.scan.before_first()
    }

    fn next(&self) -> DbResult<bool> {
        while self.scan.next()? {
            if self.predicate.is_satisfied(&self.scan)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        self.scan.get_i32(field_name)
    }

    fn get_string(&self, field_name: &Element) -> DbResult<String> {
        self.scan.get_string(field_name)
    }

    fn get_val(&self, field_name: &Element) -> common::DbResult<crate::value::Value> {
        self.scan.get_val(field_name)
    }

    fn has_field(&self, field_name: &Element) -> common::DbResult<bool> {
        self.scan.has_field(field_name)
    }

    fn close(&self) -> DbResult<()> {
        self.scan.close()
    }

    fn schema(&self) -> DbResult<Schema> {
        self.scan.schema()
    }

    fn set_i32(&self, field_name: &Element, value: i32) -> DbResult<()> {
        self.scan.set_i32(field_name, value)
    }

    fn set_string(&self, field_name: &Element, value: &str) -> DbResult<()> {
        self.scan.set_string(field_name, value)
    }

    fn set_val(&self, field_name: &Element, value: crate::value::Value) -> DbResult<()> {
        self.scan.set_val(field_name, value)
    }

    fn insert(&self) -> DbResult<()> {
        self.scan.insert()
    }

    fn delete(&self) -> DbResult<()> {
        self.scan.delete()
    }

    fn get_rid(&self) -> DbResult<crate::rid::RID> {
        self.scan.get_rid()
    }

    fn move_to_rid(&self, rid: crate::rid::RID) -> DbResult<()> {
        self.scan.move_to_rid(rid)
    }
}
