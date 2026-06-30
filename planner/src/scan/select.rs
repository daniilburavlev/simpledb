use common::DbResult;

use crate::{
    element::Element, predicate::Predicate, rid::RID, scan::Scan, schema::Schema, value::Value,
};

pub(crate) struct SelectScan {
    scan: Box<Scan>,
    predicate: Predicate,
}

impl SelectScan {
    pub(crate) fn new(scan: Box<Scan>, predicate: Predicate) -> Self {
        Self { scan, predicate }
    }

    pub(crate) fn before_first(&mut self) -> DbResult<()> {
        self.scan.before_first()
    }

    pub(crate) fn next(&mut self) -> DbResult<bool> {
        while self.scan.next_row()? {
            if self.predicate.is_satisfied(&self.scan)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        self.scan.get_i32(field_name)
    }

    pub(crate) fn get_string(&self, field_name: &Element) -> DbResult<String> {
        self.scan.get_string(field_name)
    }

    pub(crate) fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        self.scan.get_val(field_name)
    }

    pub(crate) fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        self.scan.has_field(field_name)
    }

    pub(crate) fn close(&self) -> DbResult<()> {
        self.scan.close()
    }

    pub(crate) fn schema(&self) -> DbResult<Schema> {
        self.scan.schema()
    }

    pub(crate) fn set_i32(&self, field_name: &Element, value: i32) -> DbResult<()> {
        self.scan.set_i32(field_name, value)
    }

    pub(crate) fn set_string(&self, field_name: &Element, value: &str) -> DbResult<()> {
        self.scan.set_string(field_name, value)
    }

    pub(crate) fn set_val(&self, field_name: &Element, value: Value) -> DbResult<()> {
        self.scan.set_val(field_name, value)
    }

    pub(crate) fn insert(&mut self) -> DbResult<()> {
        self.scan.insert()
    }

    pub(crate) fn delete(&self) -> DbResult<()> {
        self.scan.delete()
    }

    pub(crate) fn get_rid(&self) -> DbResult<RID> {
        self.scan.get_rid()
    }

    pub(crate) fn move_to_rid(&mut self, rid: RID) -> DbResult<()> {
        self.scan.move_to_rid(rid)
    }
}
