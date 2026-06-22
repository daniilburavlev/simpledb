use std::{collections::HashMap, rc::Rc};

use common::{DbResult, error::DbError, locks::TimedRwLock};

use crate::{constant::Constant, group::aggregation_fn::AggregationFn, scan::Scan};

#[derive(PartialEq, Eq)]
struct GroupValue {
    values: HashMap<String, Constant>,
}

impl GroupValue {
    fn new(scan: &Rc<dyn Scan>, fields: Vec<String>) -> DbResult<Self> {
        let mut values = HashMap::new();
        for field in fields {
            let value = scan.get_val(&field)?;
            values.insert(field, value);
        }
        Ok(Self { values })
    }

    fn get_val(&self, field: &str) -> DbResult<Constant> {
        match self.values.get(field) {
            Some(value) => Ok(value.clone()),
            _ => Err(DbError::field_not_exists(field)),
        }
    }
}

struct GroupByScanLock {
    scan: Rc<dyn Scan>,
    group_fields: Vec<String>,
    agg_fns: Vec<AggregationFn>,
    group_val: GroupValue,
    more_groups: bool,
}

impl GroupByScanLock {
    fn new(
        scan: &Rc<dyn Scan>,
        group_fields: Vec<String>,
        agg_fns: Vec<AggregationFn>,
    ) -> DbResult<Self> {
        let mut g = Self {
            scan: Rc::clone(scan),
            group_fields,
            agg_fns,
            group_val: GroupValue::new(scan, vec![])?,
            more_groups: false,
        };
        g.before_first()?;
        Ok(g)
    }

    fn before_first(&mut self) -> DbResult<()> {
        self.scan.before_first()?;
        self.more_groups = self.scan.next()?;
        Ok(())
    }

    fn next(&mut self) -> DbResult<bool> {
        if !self.more_groups {
            return Ok(false);
        }
        for f in &mut self.agg_fns {
            f.process_first(&self.scan)?;
        }
        self.group_val = GroupValue::new(&self.scan, self.group_fields.clone())?;
        loop {
            self.more_groups = self.scan.next()?;
            if !self.more_groups {
                break;
            }
            let gv = GroupValue::new(&self.scan, self.group_fields.clone())?;
            if gv != self.group_val {
                break;
            }
            for f in &mut self.agg_fns {
                f.process_next(&self.scan)?;
            }
        }
        Ok(true)
    }

    fn close(&self) -> DbResult<()> {
        self.scan.close()
    }

    fn get_val(&self, field_name: &str) -> DbResult<Constant> {
        if self.group_fields.contains(&field_name.to_string()) {
            return self.group_val.get_val(field_name);
        }
        for f in &self.agg_fns {
            if f.field_name() == field_name {
                return Ok(f.value());
            }
        }
        Err(DbError::field_not_exists(field_name))
    }

    fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        self.get_val(field_name)?.as_i32()
    }

    fn get_string(&self, field_name: &str) -> DbResult<String> {
        self.get_val(field_name)?.as_string()
    }

    fn has_field(&self, field_name: &str) -> bool {
        if self.group_fields.contains(&field_name.to_string()) {
            return true;
        }
        for f in &self.agg_fns {
            if f.field_name() == field_name {
                return true;
            }
        }
        false
    }
}

pub struct GroupByScan {
    lock: TimedRwLock<GroupByScanLock>,
}

impl GroupByScan {
    pub fn new(
        scan: &Rc<dyn Scan>,
        group_fields: Vec<String>,
        agg_fns: Vec<AggregationFn>,
    ) -> DbResult<Self> {
        Ok(Self {
            lock: TimedRwLock::new(GroupByScanLock::new(scan, group_fields, agg_fns)?),
        })
    }
}

impl Scan for GroupByScan {
    fn before_first(&self) -> DbResult<()> {
        let mut write = self.lock.write()?;
        write.before_first()
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
        Ok(read.has_field(field_name))
    }

    fn close(&self) -> DbResult<()> {
        let read = self.lock.read()?;
        read.close()
    }
}
