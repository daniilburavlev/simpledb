use std::sync::Arc;

use common::{DbResult, error::DbError};
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::element::Element;
use crate::schema::Schema;
use crate::value::Value;
use crate::{field_info::FieldInfo, layout::Layout, record_page::RecordPage};

pub(crate) struct ChunkScan {
    buffers: Vec<RecordPage>,
    tx: Arc<Transaction>,
    filename: String,
    layout: Layout,
    start_b_num: i32,
    end_b_num: i32,
    current_b_num: i32,
    rp: i32,
    current_slot: i32,
}

impl ChunkScan {
    pub(crate) fn new(
        tx: &Arc<Transaction>,
        filename: &str,
        layout: Layout,
        start_b_num: i32,
        end_b_num: i32,
    ) -> DbResult<Self> {
        let mut buffers = vec![];
        for i in start_b_num..=end_b_num {
            let block = BlockId::new(filename, i);
            buffers.push(RecordPage::new(tx, block, layout.clone())?);
        }
        let chunk = Self {
            buffers,
            tx: Arc::clone(tx),
            filename: filename.to_string(),
            start_b_num,
            end_b_num,
            current_b_num: start_b_num,
            layout,
            current_slot: -1,
            rp: 0,
        };
        Ok(chunk)
    }

    pub(crate) fn move_to_block(&mut self, block_num: i32) {
        self.current_b_num = block_num;
        self.rp = self.current_b_num - self.start_b_num;
        self.current_slot = -1;
    }

    pub(crate) fn close(&self) -> DbResult<()> {
        for i in 0..self.buffers.len() {
            let block = BlockId::new(&self.filename, self.start_b_num + i as i32);
            self.tx.unpin(&block)?;
        }
        Ok(())
    }

    pub(crate) fn before_first(&mut self) {
        self.move_to_block(self.start_b_num);
    }

    pub(crate) fn next(&mut self) -> DbResult<bool> {
        loop {
            let Some(rp) = self.buffers.get(self.rp as usize).cloned() else {
                return Err(DbError::other("cannot get record page for chunk"));
            };
            self.current_slot = rp.next_after(self.current_slot)?;
            if self.current_slot >= 0 {
                return Ok(true);
            }
            if self.current_b_num == self.end_b_num {
                return Ok(false);
            }
            self.move_to_block(rp.block().num + 1);
        }
    }

    pub(crate) fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        if let Some(rp) = self.buffers.get(self.rp as usize) {
            rp.get_i32(self.current_slot, field_name)
        } else {
            Err(DbError::other("cannot get buffer chunk"))
        }
    }

    pub(crate) fn get_string(&self, field_name: &Element) -> DbResult<String> {
        if let Some(rp) = self.buffers.get(self.rp as usize) {
            rp.get_string(self.current_slot, field_name)
        } else {
            Err(DbError::other("cannot get buffer chunk"))
        }
    }

    pub(crate) fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        match self.layout.schema().info(field_name) {
            Some(FieldInfo::Integer) => Ok(Value::Integer(self.get_i32(field_name)?)),
            Some(FieldInfo::Varchar(_)) => Ok(Value::Varchar(self.get_string(field_name)?)),
            _ => Err(DbError::FieldNotExists(field_name.to_string())),
        }
    }

    pub(crate) fn has_field(&self, field_name: &Element) -> bool {
        self.layout.schema().has_field(field_name)
    }

    pub(crate) fn schema(&self) -> Schema {
        self.layout.schema().clone()
    }
}
