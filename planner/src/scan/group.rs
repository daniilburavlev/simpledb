use std::{cell::RefCell, cmp::Ordering, collections::HashMap, rc::Rc, sync::Arc};

use common::{DbResult, error::DbError};

use crate::element::Element;
use crate::value::Value;
use crate::{scan::Scan, schema::Schema};

#[allow(dead_code)]
#[derive(Clone)]
pub enum AggregationFn {
    MaxFn { field: Element, value: Value },
}

impl AggregationFn {
    pub fn process_first(&mut self, scan: &Scan) -> DbResult<()> {
        match self {
            Self::MaxFn { field, value } => {
                *value = scan.get_val(field)?;
            }
        }
        Ok(())
    }

    pub fn process_next(&mut self, scan: &Scan) -> DbResult<()> {
        match self {
            Self::MaxFn { field, value } => {
                let new_value = scan.get_val(field)?;
                if new_value.cmp(value) == Ordering::Greater {
                    *value = new_value;
                }
            }
        }
        Ok(())
    }

    pub fn field_name(&self) -> &Element {
        match self {
            Self::MaxFn { field, .. } => field,
        }
    }

    pub fn value(&self) -> Value {
        match self {
            Self::MaxFn { value, .. } => value.clone(),
        }
    }
}

#[derive(PartialEq, Eq)]
struct GroupValue {
    values: HashMap<Element, Value>,
}

impl GroupValue {
    fn new(scan: &Scan, fields: Vec<Element>) -> DbResult<Self> {
        let mut values = HashMap::new();
        for field in fields {
            let value = scan.get_val(&field)?;
            values.insert(field, value);
        }
        Ok(Self { values })
    }

    fn get_val(&self, field: &Element) -> DbResult<Value> {
        match self.values.get(field) {
            Some(value) => Ok(value.clone()),
            _ => Err(DbError::FieldNotExists(field.to_string())),
        }
    }
}

struct GroupByScan {
    scan: Box<Scan>,
    group_fields: Vec<Element>,
    agg_fns: Vec<AggregationFn>,
    group_val: GroupValue,
    more_groups: bool,
}

impl GroupByScan {
    fn new(
        scan: Box<Scan>,
        group_fields: Vec<Element>,
        agg_fns: Vec<AggregationFn>,
    ) -> DbResult<Self> {
        let mut g = Self {
            group_val: GroupValue::new(&scan, vec![])?,
            scan,
            group_fields,
            agg_fns,
            more_groups: false,
        };
        g.before_first()?;
        Ok(g)
    }

    fn before_first(&mut self) -> DbResult<()> {
        self.scan.before_first()?;
        self.more_groups = self.scan.next_row()?;
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
            self.more_groups = self.scan.next_row()?;
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

    fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        if self.group_fields.contains(&field_name) {
            return self.group_val.get_val(field_name);
        }
        for f in &self.agg_fns {
            if f.field_name() == field_name {
                return Ok(f.value());
            }
        }
        Err(DbError::FieldNotExists(field_name.to_string()))
    }

    fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        self.get_val(field_name)?.as_i32()
    }

    fn get_string(&self, field_name: &Element) -> DbResult<String> {
        self.get_val(field_name)?.as_string()
    }

    fn has_field(&self, field_name: &Element) -> bool {
        if self.group_fields.contains(field_name) {
            return true;
        }
        for f in &self.agg_fns {
            if f.field_name() == field_name {
                return true;
            }
        }
        false
    }

    fn schema(&self) -> DbResult<Schema> {
        self.scan.schema()
    }
}
