use common::{DbResult, locks::TimedRwLock};
use std::sync::Arc;
use std::{cmp::Ordering, rc::Rc};

use crate::schema::Schema;
use crate::{constant::Constant, scan::Scan};

struct MergeJoinScanLock {
    s1: Rc<dyn Scan>,
    s2: Rc<dyn Scan>,
    field_name1: String,
    field_name2: String,
    join_val: Option<Constant>,
}

impl MergeJoinScanLock {
    pub fn new(
        s1: &Rc<dyn Scan>,
        s2: &Rc<dyn Scan>,
        field_name1: &str,
        field_name2: &str,
    ) -> DbResult<Self> {
        let s = Self {
            s1: Rc::clone(s1),
            s2: Rc::clone(s2),
            field_name1: field_name1.to_string(),
            field_name2: field_name2.to_string(),
            join_val: None,
        };
        s.before_first()?;
        Ok(s)
    }

    fn before_first(&self) -> DbResult<()> {
        self.s1.before_first()?;
        self.s2.before_first()
    }

    fn next(&mut self) -> DbResult<bool> {
        let Some(join_val) = &self.join_val else {
            return Ok(false);
        };
        let mut has_more2 = self.s2.next()?;
        if has_more2 && self.s2.get_val(&self.field_name2)? == *join_val {
            return Ok(true);
        }
        let mut has_more1 = self.s1.next()?;
        if has_more1 && self.s1.get_val(&self.field_name1)? == *join_val {
            return Ok(true);
        }

        while has_more1 && has_more2 {
            let v1 = self.s1.get_val(&self.field_name1)?;
            let v2 = self.s2.get_val(&self.field_name2)?;
            match v1.cmp(&v2) {
                Ordering::Less => has_more1 = self.s1.next()?,
                Ordering::Greater => has_more2 = self.s1.next()?,
                _ => {
                    self.s2.save_position()?;
                    self.join_val = Some(self.s2.get_val(&self.field_name2)?);
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        if self.s1.has_field(field_name)? {
            self.s1.get_i32(field_name)
        } else {
            self.s2.get_i32(field_name)
        }
    }

    fn get_string(&self, field_name: &str) -> DbResult<String> {
        if self.s1.has_field(field_name)? {
            self.s1.get_string(field_name)
        } else {
            self.s2.get_string(field_name)
        }
    }

    fn get_val(&self, field_name: &str) -> DbResult<crate::constant::Constant> {
        if self.s1.has_field(field_name)? {
            self.s1.get_val(field_name)
        } else {
            self.s2.get_val(field_name)
        }
    }

    fn has_field(&self, field_name: &str) -> DbResult<bool> {
        Ok(self.s1.has_field(field_name)? || self.s2.has_field(field_name)?)
    }

    fn close(&self) -> DbResult<()> {
        self.s1.close()?;
        self.s2.close()
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        let s1 = self.s1.schema()?;
        let s2 = self.s2.schema()?;
        s1.add_all(&s2)?;
        Ok(s1)
    }
}

pub struct MergeJoinScan {
    lock: TimedRwLock<MergeJoinScanLock>,
}

impl MergeJoinScan {
    pub fn new(
        s1: &Rc<dyn Scan>,
        s2: &Rc<dyn Scan>,
        field_name1: &str,
        field_name2: &str,
    ) -> DbResult<Self> {
        Ok(Self {
            lock: TimedRwLock::new(MergeJoinScanLock::new(s1, s2, field_name1, field_name2)?),
        })
    }
}

impl Scan for MergeJoinScan {
    fn before_first(&self) -> DbResult<()> {
        let read = self.lock.read()?;
        read.before_first()
    }

    fn next(&self) -> DbResult<bool> {
        let mut write = self.lock.write()?;
        write.next()
    }

    fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        let read = self.lock.read()?;
        read.get_i32(field_name)
    }

    fn get_string(&self, field_name: &str) -> DbResult<String> {
        let read = self.lock.read()?;
        read.get_string(field_name)
    }

    fn get_val(&self, field_name: &str) -> DbResult<Constant> {
        let read = self.lock.read()?;
        read.get_val(field_name)
    }

    fn has_field(&self, field_name: &str) -> DbResult<bool> {
        let read = self.lock.read()?;
        read.has_field(field_name)
    }

    fn close(&self) -> DbResult<()> {
        let read = self.lock.read()?;
        read.close()
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        let read = self.lock.read()?;
        read.schema()
    }
}
