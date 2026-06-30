use crate::element::Element;
use crate::scan::Scan;
use crate::schema::{Schema, SchemaBuilder};
use crate::value::Value;
use common::DbResult;

pub struct ProductScan {
    s1: Box<Scan>,
    s2: Box<Scan>,
}

impl ProductScan {
    pub fn new(mut s1: Box<Scan>, s2: Box<Scan>) -> DbResult<Self> {
        s1.next_row()?;
        Ok(Self { s1, s2 })
    }
}

impl ProductScan {
    pub(crate) fn before_first(&mut self) -> DbResult<()> {
        self.s1.before_first()?;
        self.s1.next_row()?;
        self.s2.before_first()
    }

    pub(crate) fn next_row(&mut self) -> DbResult<bool> {
        if self.s2.next_row()? {
            Ok(true)
        } else {
            self.s2.before_first()?;
            Ok(self.s2.next_row()? && self.s1.next_row()?)
        }
    }

    pub(crate) fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        if self.s1.has_field(field_name)? {
            self.s1.get_i32(field_name)
        } else {
            self.s2.get_i32(field_name)
        }
    }

    pub(crate) fn get_string(&self, field_name: &Element) -> DbResult<String> {
        if self.s1.has_field(field_name)? {
            self.s1.get_string(field_name)
        } else {
            self.s2.get_string(field_name)
        }
    }

    pub(crate) fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        if self.s1.has_field(field_name)? {
            self.s1.get_val(field_name)
        } else {
            self.s2.get_val(field_name)
        }
    }

    pub(crate) fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        Ok(self.s1.has_field(field_name)? || self.s2.has_field(field_name)?)
    }

    pub(crate) fn close(&self) -> DbResult<()> {
        self.s1.close()?;
        self.s2.close()
    }

    pub(crate) fn schema(&self) -> DbResult<Schema> {
        let s1 = self.s1.schema()?;
        let s2 = self.s2.schema()?;
        let s = SchemaBuilder::default().add_all(&s1).add_all(&s2).build();
        Ok(s)
    }
}
