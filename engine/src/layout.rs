use std::{collections::HashMap, sync::Arc};

use common::DbResult;
use file::page::I32_SIZE;

use crate::schema::Schema;

pub struct Layout {
    schema: Arc<Schema>,
    offsets: HashMap<String, i32>,
    slotsize: i32,
}

impl Layout {
    pub fn new(schema: &Arc<Schema>) -> DbResult<Self> {
        let mut offsets = HashMap::new();
        let mut pos = I32_SIZE as i32;
        for (field, info) in schema.fields()? {
            offsets.insert(field, pos);
            pos += info.length();
        }
        Ok(Self {
            schema: Arc::clone(schema),
            offsets,
            slotsize: pos,
        })
    }

    pub fn from(schema: &Arc<Schema>, offsets: HashMap<String, i32>, slotsize: i32) -> Self {
        Self {
            schema: Arc::clone(schema),
            offsets,
            slotsize,
        }
    }

    pub fn offset(&self, fieldname: &str) -> i32 {
        if let Some(offset) = self.offsets.get(fieldname) {
            *offset
        } else {
            0
        }
    }

    pub fn schema(&self) -> Arc<Schema> {
        Arc::clone(&self.schema)
    }

    pub fn slotsize(&self) -> i32 {
        self.slotsize
    }
}
