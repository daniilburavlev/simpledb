use crate::element::Element;
use crate::rid::RID;
use crate::scan::Scan;
use crate::schema::{Schema, SchemaBuilder};
use crate::temp::TempTable;
use crate::value::Value;
use common::{DbResult, error::DbError};
use std::{cell::RefCell, cmp::Ordering, rc::Rc};

#[derive(Clone)]
pub struct RecordComparator {
    fields: Vec<Element>,
}

impl RecordComparator {
    pub fn new(fields: Vec<Element>) -> Self {
        Self { fields }
    }

    pub fn compare(&self, s1: &Rc<dyn Scan>, s2: &Rc<dyn Scan>) -> DbResult<Ordering> {
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

pub struct OrderScanInner {
    s1: Rc<dyn Scan>,
    s2: Option<Rc<dyn Scan>>,
    current_scan: CurrentScan,
    comp: RecordComparator,
    has_more1: bool,
    has_more2: bool,
    saved_position: Vec<RID>,
}

impl OrderScanInner {
    fn new(runs: Vec<TempTable>, comp: RecordComparator) -> DbResult<Self> {
        let s1 = runs[0].open()?;
        let (s2, has_more2) = if runs.len() > 1 {
            let s2 = runs[1].open()?;
            let has_more2 = s2.next()?;
            (Some(s2), has_more2)
        } else {
            (None, false)
        };
        Ok(Self {
            comp,
            has_more1: s1.next()?,
            has_more2,
            s1,
            s2,
            current_scan: CurrentScan::None,
            saved_position: vec![],
        })
    }
}

impl OrderScanInner {
    fn before_first(&mut self) -> DbResult<()> {
        self.s1.before_first()?;
        self.has_more1 = self.s1.next()?;
        if let Some(s2) = &mut self.s2 {
            s2.before_first()?;
            self.has_more2 = s2.next()?;
        }
        Ok(())
    }

    fn next(&mut self) -> DbResult<bool> {
        match self.current_scan {
            CurrentScan::S1 => self.has_more1 = self.s1.next()?,
            CurrentScan::S2 if let Some(s2) = &self.s2 => self.has_more2 = s2.next()?,
            _ => {}
        }
        if !self.has_more1 && !self.has_more2 {
            return Ok(false);
        } else if self.has_more1
            && self.has_more2
            && let Some(s2) = &self.s2
        {
            if self.comp.compare(&self.s1, s2)? == Ordering::Less {
                self.current_scan = CurrentScan::S1;
            } else {
                self.current_scan = CurrentScan::S2;
            }
        } else if self.has_more1 {
            self.current_scan = CurrentScan::S1;
        } else if self.has_more2 {
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
            CurrentScan::S2 if let Some(s2) = &self.s2 => s2.get_val(field_name),
            _ => Err(DbError::other("invalid scan")),
        }
    }

    fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        match self.current_scan {
            CurrentScan::S1 => self.s1.get_i32(field_name),
            CurrentScan::S2 if let Some(s2) = &self.s2 => s2.get_i32(field_name),
            _ => Err(DbError::other("invalid scan")),
        }
    }

    fn get_string(&self, field_name: &Element) -> DbResult<String> {
        match self.current_scan {
            CurrentScan::S1 => self.s1.get_string(field_name),
            CurrentScan::S2 if let Some(s2) = &self.s2 => s2.get_string(field_name),
            _ => Err(DbError::other("invalid scan")),
        }
    }

    fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        match self.current_scan {
            CurrentScan::S1 => self.s1.has_field(field_name),
            CurrentScan::S2 if let Some(s2) = &self.s2 => s2.has_field(field_name),
            _ => Err(DbError::other("invalid scan")),
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

    fn restore_position(&self) -> DbResult<()> {
        if let Some(rid1) = self.saved_position.first().cloned() {
            self.s1.move_to_rid(rid1)?;
        }
        if let Some(rid2) = self.saved_position.get(1).cloned()
            && let Some(s2) = &self.s2
        {
            s2.move_to_rid(rid2)?;
        }
        Ok(())
    }

    fn schema(&self) -> DbResult<Schema> {
        let s1 = self.s1.schema()?;
        let mut s = SchemaBuilder::new(Element::raw("group"));
        s = s.add_all(&s1);
        if let Some(s2) = &self.s2 {
            let s2 = s2.schema()?;
            s = s.add_all(&s2);
        }
        let s = s.build();
        Ok(s)
    }
}

pub struct OrderScan {
    lock: RefCell<OrderScanInner>,
}

impl OrderScan {
    pub fn new(runs: Vec<TempTable>, comp: RecordComparator) -> DbResult<Self> {
        Ok(Self {
            lock: RefCell::new(OrderScanInner::new(runs, comp)?),
        })
    }

    #[allow(dead_code)]
    pub fn save_position(&self) -> DbResult<()> {
        let mut write = self.lock.borrow_mut();
        write.save_position()
    }

    #[allow(dead_code)]
    pub fn restore_position(&self) -> DbResult<()> {
        let read = self.lock.borrow();
        read.restore_position()
    }
}

impl Scan for OrderScan {
    fn before_first(&self) -> DbResult<()> {
        let mut write = self.lock.borrow_mut();
        write.before_first()
    }

    fn next(&self) -> DbResult<bool> {
        let mut write = self.lock.borrow_mut();
        write.next()
    }

    fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        let read = self.lock.borrow();
        read.get_i32(field_name)
    }

    fn get_string(&self, field_name: &Element) -> DbResult<String> {
        let read = self.lock.borrow();
        read.get_string(field_name)
    }

    fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        let read = self.lock.borrow();
        read.get_val(field_name)
    }

    fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        let read = self.lock.borrow();
        read.has_field(field_name)
    }

    fn close(&self) -> DbResult<()> {
        let read = self.lock.borrow();
        read.close()
    }

    fn schema(&self) -> DbResult<Schema> {
        let read = self.lock.borrow();
        read.schema()
    }
}
