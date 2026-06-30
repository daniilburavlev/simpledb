use crate::element::Element;
use crate::scan::Scan;
use crate::schema::{Schema, SchemaBuilder};
use common::DbResult;
use std::rc::Rc;
use std::sync::Arc;

pub struct ProductScan {
    s1: Rc<dyn Scan>,
    s2: Rc<dyn Scan>,
}

impl ProductScan {
    pub fn new(s1: Rc<dyn Scan>, s2: Rc<dyn Scan>) -> DbResult<Self> {
        s1.next()?;
        Ok(Self { s1, s2 })
    }
}

impl Scan for ProductScan {
    fn before_first(&self) -> DbResult<()> {
        self.s1.before_first()?;
        self.s1.next()?;
        self.s2.before_first()
    }

    fn next(&self) -> DbResult<bool> {
        if self.s2.next()? {
            Ok(true)
        } else {
            self.s2.before_first()?;
            Ok(self.s2.next()? && self.s1.next()?)
        }
    }

    fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        if self.s1.has_field(field_name)? {
            self.s1.get_i32(field_name)
        } else {
            self.s2.get_i32(field_name)
        }
    }

    fn get_string(&self, field_name: &Element) -> DbResult<String> {
        if self.s1.has_field(field_name)? {
            self.s1.get_string(field_name)
        } else {
            self.s2.get_string(field_name)
        }
    }

    fn get_val(&self, field_name: &Element) -> DbResult<crate::value::Value> {
        if self.s1.has_field(field_name)? {
            self.s1.get_val(field_name)
        } else {
            self.s2.get_val(field_name)
        }
    }

    fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        Ok(self.s1.has_field(field_name)? || self.s2.has_field(field_name)?)
    }

    fn close(&self) -> DbResult<()> {
        self.s1.close()?;
        self.s2.close()
    }

    fn schema(&self) -> DbResult<Schema> {
        let s1 = self.s1.schema()?;
        let s2 = self.s2.schema()?;
        let s = SchemaBuilder::default().add_all(&s1).add_all(&s2).build();
        Ok(s)
    }
}
