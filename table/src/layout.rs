use std::{collections::HashMap, sync::Arc};

use common::DbResult;
use file::page::U8_SIZE;

use crate::schema::Schema;

pub struct Layout {
    schema: Arc<Schema>,
    offsets: HashMap<String, u16>,
    slotsize: u16,
}

impl Layout {
    pub fn new(schema: &Arc<Schema>) -> DbResult<Self> {
        let mut offsets = HashMap::new();
        let mut pos = U8_SIZE as u16;
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

    pub fn from(schema: &Arc<Schema>, offsets: HashMap<String, u16>, slotsize: u16) -> Self {
        Self {
            schema: Arc::clone(schema),
            offsets,
            slotsize,
        }
    }

    pub fn offset(&self, fieldname: &str) -> u16 {
        if let Some(offset) = self.offsets.get(fieldname) {
            *offset
        } else {
            0
        }
    }

    pub fn schema(&self) -> Arc<Schema> {
        Arc::clone(&self.schema)
    }

    pub fn slotsize(&self) -> u16 {
        self.slotsize
    }
}
