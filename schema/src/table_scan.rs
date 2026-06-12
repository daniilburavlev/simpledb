use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

use common::DbResult;
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::{layout::Layout, record_page::RecordPage};

pub struct TableScan {
    tx: Arc<Transaction>,
    layout: Arc<Layout>,
    rp: RefCell<RecordPage>,
    filename: String,
    current_slot: Cell<i32>,
}

impl TableScan {
    pub fn new(tx: &Arc<Transaction>, table_name: &str, layout: &Arc<Layout>) -> DbResult<Self> {
        let filename = format!("{}.tbl", table_name);
        let table_scan = Self {
            tx: Arc::clone(tx),
            layout: Arc::clone(layout),
            rp: RefCell::new(RecordPage::new(tx, BlockId::new("", 0), layout)?),
            filename: filename.clone(),
            current_slot: Cell::new(0),
        };
        if tx.size(&filename)? == 0 {
            table_scan.move_to_new_block()?;
        } else {
            table_scan.move_to_block(0)?;
        }
        Ok(table_scan)
    }

    pub fn close(&self) -> DbResult<()> {
        let rp = self.rp.borrow();
        self.tx.unpin(&rp.block())
    }

    pub fn before_first(&self) -> DbResult<()> {
        self.move_to_block(0)
    }

    pub fn next(&self) -> DbResult<bool> {
        let rp = self.rp.borrow();
        let mut current_slot = self.current_slot.get();
        current_slot = rp.next_after(current_slot)?;
        while current_slot < 0 {
            if self.at_last_block()? {
                return Ok(false);
            }
            self.move_to_block(rp.block().num + 1)?;
            current_slot = rp.next_after(current_slot)?;
        }
        self.current_slot.replace(current_slot);
        Ok(true)
    }

    pub fn get_i32(&self, filename: &str) -> DbResult<i32> {
        self.rp.borrow().get_i32(self.current_slot.get(), filename)
    }

    pub fn get_string(&self, filename: &str) -> DbResult<String> {
        self.rp
            .borrow()
            .get_string(self.current_slot.get(), filename)
    }

    pub fn get_val(&self, fieldname: &str) -> DbResult<Constant> {
        let field_type = self.layout.schema().info(fieldname)?
    }

    fn move_to_block(&self, num: i32) -> DbResult<()> {
        self.close()?;
        let block = BlockId::new(&self.filename, num);
        self.rp
            .replace(RecordPage::new(&self.tx, block, &self.layout)?);
        self.current_slot.replace(-1);
        Ok(())
    }

    fn move_to_new_block(&self) -> DbResult<()> {
        self.close()?;
        let block = self.tx.append(&self.filename)?;
        self.rp
            .replace(RecordPage::new(&self.tx, block, &self.layout)?);
        self.rp.borrow().format()?;
        self.current_slot.replace(-1);
        Ok(())
    }

    fn at_last_block(&self) -> DbResult<bool> {
        Ok(self.rp.borrow().block().num == self.tx.size(&self.filename)? as i32 - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
