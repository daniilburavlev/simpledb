use std::sync::Arc;

use common::{DbResult, error::DbError};
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::schema::Schema;
use crate::{
    constant::Constant, field_info::FieldInfo, layout::Layout, record_page::RecordPage, scan::Scan,
};

pub(crate) struct ChunkScan {
    buffers: Vec<RecordPage>,
    tx: Arc<Transaction>,
    filename: String,
    layout: Arc<Layout>,
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
        layout: &Arc<Layout>,
        start_b_num: i32,
        end_b_num: i32,
    ) -> DbResult<Self> {
        let mut buffers = vec![];
        for i in start_b_num..=end_b_num {
            let block = BlockId::new(filename, i);
            buffers.push(RecordPage::new(tx, block, layout)?);
        }
        let chunk = Self {
            buffers,
            tx: Arc::clone(tx),
            filename: filename.to_string(),
            start_b_num,
            end_b_num,
            current_b_num: start_b_num,
            layout: Arc::clone(layout),
            current_slot: -1,
            rp: 0,
        };
        Ok(chunk)
    }

    fn move_to_block(&mut self, block_num: i32) {
        self.current_b_num = block_num;
        self.rp = self.current_b_num - self.start_b_num;
        self.current_slot = -1;
    }
}

impl Scan for ChunkScan {
    fn close(&self) -> DbResult<()> {
        for _ in 0..self.buffers.len() {
            let block = BlockId::new(&self.filename, self.start_b_num + 1);
            self.tx.unpin(&block)?;
        }
        Ok(())
    }

    fn before_first(&mut self) -> DbResult<()> {
        self.move_to_block(self.start_b_num);
        Ok(())
    }

    fn next(&mut self) -> DbResult<bool> {
        let Some(rp) = self.buffers.get(self.current_slot as usize).cloned() else {
            return Err(DbError::other("cannot get record page for chunk"));
        };
        self.current_slot = rp.next_after(self.current_slot)?;
        while self.current_slot < 0 {
            if self.current_b_num == self.end_b_num {
                return Ok(false);
            }
            self.move_to_block(rp.block().num + 1);
            self.current_slot = rp.next_after(self.current_slot)?;
        }
        Ok(true)
    }

    fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        if let Some(rp) = self.buffers.get(self.current_b_num as usize) {
            rp.get_i32(self.current_slot, field_name)
        } else {
            Err(DbError::other("cannot get buffer chunk"))
        }
    }

    fn get_string(&self, field_name: &str) -> DbResult<String> {
        if let Some(rp) = self.buffers.get(self.current_b_num as usize) {
            rp.get_string(self.current_slot, field_name)
        } else {
            Err(DbError::other("cannot get buffer chunk"))
        }
    }

    fn get_val(&self, field_name: &str) -> DbResult<Constant> {
        match self.layout.schema().info(field_name)? {
            Some(FieldInfo::Integer) => Ok(Constant::Integer(self.get_i32(field_name)?)),
            Some(FieldInfo::Varchar(_)) => Ok(Constant::Varchar(self.get_string(field_name)?)),
            _ => Err(DbError::field_not_exists(field_name)),
        }
    }

    fn has_field(&self, field_name: &str) -> DbResult<bool> {
        self.layout.schema().has_field(field_name)
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        Ok(self.layout.schema())
    }
}
