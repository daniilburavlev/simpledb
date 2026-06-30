use crate::element::Element;
use crate::scan::Scan;
use crate::schema::{Schema, SchemaBuilder};
use crate::value::Value;
use common::DbResult;
use std::cmp::Ordering;

#[allow(dead_code)]
struct MergeJoinScan {
    s1: Box<Scan>,
    s2: Box<Scan>,
    field_name1: Element,
    field_name2: Element,
    join_val: Option<Value>,
}

#[allow(dead_code)]
impl MergeJoinScan {
    pub fn new(
        s1: Box<Scan>,
        s2: Box<Scan>,
        field_name1: Element,
        field_name2: Element,
    ) -> DbResult<Self> {
        let mut s = Self {
            s1,
            s2,
            field_name1,
            field_name2,
            join_val: None,
        };
        s.before_first()?;
        Ok(s)
    }

    pub(crate) fn before_first(&mut self) -> DbResult<()> {
        self.s1.before_first()?;
        self.s2.before_first()
    }

    pub(crate) fn next(&mut self) -> DbResult<bool> {
        let Some(join_val) = &self.join_val else {
            return Ok(false);
        };
        let mut has_more2 = self.s2.next_row()?;
        if has_more2 && self.s2.get_val(&self.field_name2)? == *join_val {
            return Ok(true);
        }
        let mut has_more1 = self.s1.next_row()?;
        if has_more1 && self.s1.get_val(&self.field_name1)? == *join_val {
            return Ok(true);
        }

        while has_more1 && has_more2 {
            let v1 = self.s1.get_val(&self.field_name1)?;
            let v2 = self.s2.get_val(&self.field_name2)?;
            match v1.cmp(&v2) {
                Ordering::Less => has_more1 = self.s1.next_row()?,
                Ordering::Greater => has_more2 = self.s1.next_row()?,
                _ => {
                    self.s2.save_position()?;
                    self.join_val = Some(self.s2.get_val(&self.field_name2)?);
                    return Ok(true);
                }
            }
        }
        Ok(false)
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

    fn get_val(&self, field_name: &Element) -> DbResult<Value> {
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
