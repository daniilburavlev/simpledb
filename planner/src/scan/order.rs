use common::{DbResult, error::DbError};
use std::cmp::Ordering;

use crate::element::Element;
use crate::rid::RID;
use crate::scan::Scan;
use crate::schema::{Schema, SchemaBuilder};
use crate::temp::TempTable;
use crate::value::Value;

#[derive(Clone)]
pub struct RecordComparator {
    fields: Vec<Element>,
}

impl RecordComparator {
    pub fn new(fields: Vec<Element>) -> Self {
        Self { fields }
    }

    pub fn compare(&self, s1: &Scan, s2: &Scan) -> DbResult<Ordering> {
        for field in &self.fields {
            let val1 = s1.get_val(field)?;
            let val2 = s2.get_val(field)?;
            let result = val1.cmp(&val2);
            if result != Ordering::Equal {
                return Ok(result);
            }
        }
        Ok(Ordering::Equal)
    }
}

enum CurrentScan {
    None,
    S1,
    S2,
}

pub struct SortScan {
    s1: Box<Scan>,
    s2: Option<Box<Scan>>,
    current_scan: CurrentScan,
    comp: RecordComparator,
    has_more1: bool,
    has_more2: bool,
    saved_position: Vec<RID>,
}

impl SortScan {
    fn new(runs: Vec<TempTable>, comp: RecordComparator) -> DbResult<Self> {
        let mut s1 = Box::new(runs[0].open()?);
        let has_more1 = s1.next_row()?;
        let (s2, has_more2) = if runs.len() > 1 {
            let mut s2 = Box::new(runs[1].open()?);
            let has_more2 = s2.next_row()?;
            (Some(s2), has_more2)
        } else {
            (None, false)
        };
        Ok(Self {
            comp,
            has_more1,
            has_more2,
            s1,
            s2,
            current_scan: CurrentScan::None,
            saved_position: vec![],
        })
    }

    fn before_first(&mut self) -> DbResult<()> {
        self.s1.before_first()?;
        self.has_more1 = self.s1.next_row()?;
        if let Some(s2) = self.s2.as_mut() {
            s2.before_first()?;
            self.has_more2 = s2.next_row()?;
        }
        Ok(())
    }

    fn next(&mut self) -> DbResult<bool> {
        match self.current_scan {
            CurrentScan::S1 => self.has_more1 = self.s1.next_row()?,
            CurrentScan::S2 => {
                if let Some(s2) = self.s2.as_mut() {
                    self.has_more2 = s2.next_row()?;
                }
            }
            CurrentScan::None => {}
        }
        if !self.has_more1 && !self.has_more2 {
            return Ok(false);
        }
        if self.has_more1 && self.has_more2 {
            if let Some(s2) = self.s2.as_ref() {
                if self.comp.compare(&self.s1, s2)? == Ordering::Less {
                    self.current_scan = CurrentScan::S1;
                } else {
                    self.current_scan = CurrentScan::S2;
                }
            } else {
                self.current_scan = CurrentScan::S1;
            }
        } else if self.has_more1 {
            self.current_scan = CurrentScan::S1;
        } else {
            self.current_scan = CurrentScan::S2;
        }
        Ok(true)
    }

    fn close(&self) -> DbResult<()> {
        self.s1.close()?;
        if let Some(s2) = &self.s2 {
            s2.close()?;
        }
        Ok(())
    }

    fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        match self.current_scan {
            CurrentScan::S1 => self.s1.get_val(field_name),
            CurrentScan::S2 => self.right()?.get_val(field_name),
            CurrentScan::None => Err(DbError::other("invalid scan")),
        }
    }

    fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        match self.current_scan {
            CurrentScan::S1 => self.s1.get_i32(field_name),
            CurrentScan::S2 => self.right()?.get_i32(field_name),
            CurrentScan::None => Err(DbError::other("invalid scan")),
        }
    }

    fn get_string(&self, field_name: &Element) -> DbResult<String> {
        match self.current_scan {
            CurrentScan::S1 => self.s1.get_string(field_name),
            CurrentScan::S2 => self.right()?.get_string(field_name),
            CurrentScan::None => Err(DbError::other("invalid scan")),
        }
    }

    fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        match self.current_scan {
            CurrentScan::S1 => self.s1.has_field(field_name),
            CurrentScan::S2 => self.right()?.has_field(field_name),
            CurrentScan::None => Err(DbError::other("invalid scan")),
        }
    }

    fn save_position(&mut self) -> DbResult<()> {
        let rid1 = self.s1.get_rid()?;
        if let Some(s2) = &self.s2 {
            let rid2 = s2.get_rid()?;
            self.saved_position = vec![rid1, rid2];
        } else {
            self.saved_position = vec![rid1];
        }
        Ok(())
    }

    fn restore_position(&mut self) -> DbResult<()> {
        if let Some(rid1) = self.saved_position.first().cloned() {
            self.s1.move_to_rid(rid1)?;
        }
        if let Some(rid2) = self.saved_position.get(1).cloned() {
            if let Some(s2) = self.s2.as_mut() {
                s2.move_to_rid(rid2)?;
            }
        }
        Ok(())
    }

    fn schema(&self) -> DbResult<Schema> {
        let mut builder = SchemaBuilder::default().add_all(&self.s1.schema()?);
        if let Some(s2) = self.s2.as_ref() {
            builder = builder.add_all(&s2.schema()?);
        }
        Ok(builder.build())
    }

    fn right(&self) -> DbResult<&Scan> {
        self.s2
            .as_deref()
            .ok_or_else(|| DbError::other("invalid scan"))
    }
}