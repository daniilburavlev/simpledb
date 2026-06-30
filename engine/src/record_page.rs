use std::sync::Arc;

use common::DbResult;
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::{element::Element, field_info::FieldInfo, layout::Layout};

const EMPTY: u8 = 0;
const USED: u8 = 1;

#[derive(Clone)]
pub struct RecordPage {
    tx: Arc<Transaction>,
    block: BlockId,
    layout: Layout,
}

impl RecordPage {
    pub fn new(tx: &Arc<Transaction>, block: BlockId, layout: Layout) -> DbResult<Self> {
        tx.pin(&block)?;
        Ok(Self {
            tx: Arc::clone(tx),
            block,
            layout,
        })
    }

    pub fn get_i32(&self, slot: i32, field: &Element) -> DbResult<i32> {
        let pos = self.offset(slot) + self.layout.offset(field);
        self.tx.get_i32(&self.block, pos as usize)
    }

    pub fn get_string(&self, slot: i32, field: &Element) -> DbResult<String> {
        let pos = self.offset(slot) + self.layout.offset(field);
        self.tx.get_string(&self.block, pos as usize)
    }

    pub fn set_i32(&self, slot: i32, field: &Element, value: i32) -> DbResult<()> {
        let pos = self.offset(slot) + self.layout.offset(field);
        self.tx.set_i32(&self.block, pos as usize, value, true)
    }

    pub fn set_string(&self, slot: i32, field: &Element, value: &str) -> DbResult<()> {
        let pos = self.offset(slot) + self.layout.offset(field);
        self.tx.set_string(&self.block, pos as usize, value, true)
    }

    pub fn delete(&self, slot: i32) -> DbResult<()> {
        self.set_flag(slot, EMPTY)
    }

    pub fn format(&self) -> DbResult<()> {
        let mut slot = 0;
        while self.is_valid_slot(slot) {
            self.tx
                .set_u8(&self.block, self.offset(slot) as usize, EMPTY, false)?;
            let schema = self.layout.schema();
            for (field, info) in schema.fields() {
                let pos = self.offset(slot) + self.layout.offset(&field);
                match info {
                    FieldInfo::Integer => self.tx.set_i32(&self.block, pos as usize, 0, false)?,
                    FieldInfo::Varchar(_) => {
                        self.tx.set_string(&self.block, pos as usize, "", false)?
                    }
                }
            }
            slot += 1;
        }
        Ok(())
    }

    pub fn next_after(&self, slot: i32) -> DbResult<i32> {
        self.search_after(slot, USED)
    }

    pub fn insert_after(&self, slot: i32) -> DbResult<i32> {
        let newslot = self.search_after(slot, EMPTY)?;
        if newslot >= 0 {
            self.set_flag(newslot, USED)?;
        }
        Ok(newslot)
    }

    pub fn block(&self) -> BlockId {
        self.block.clone()
    }

    fn set_flag(&self, slot: i32, flag: u8) -> DbResult<()> {
        let offset = self.offset(slot) as usize;
        self.tx.set_u8(&self.block, offset, flag, true)?;
        Ok(())
    }

    fn search_after(&self, mut slot: i32, flag: u8) -> DbResult<i32> {
        slot += 1;
        while self.is_valid_slot(slot) {
            if self.tx.get_u8(&self.block, self.offset(slot) as usize)? == flag {
                return Ok(slot);
            }
            slot += 1;
        }
        Ok(-1)
    }

    fn is_valid_slot(&self, slot: i32) -> bool {
        let offset = self.offset(slot + 1);
        let block_size = self.tx.block_size();
        offset <= block_size
    }

    fn offset(&self, slot: i32) -> i32 {
        slot * self.layout.slotsize()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use buffer::mgr::BufferMgr;
    use file::mgr::FileMgr;
    use log::mgr::LogMgr;
    use rand::RngExt;
    use tempfile::tempdir;
    use transaction::lock_table::LockTable;

    use crate::schema::SchemaBuilder;

    use super::*;

    #[test]
    fn record() {
        let dir = tempdir().unwrap();
        let fm = Arc::new(FileMgr::new(dir.path(), 512).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "testlog".to_string()).unwrap());
        let bm = Arc::new(BufferMgr::new(&fm, &lm, 1).unwrap());
        let lock_table = Arc::new(LockTable::default());

        let tx = Transaction::new(&fm, &lm, &bm, &lock_table).unwrap();
        let tx = Arc::new(tx);

        let a = Element::raw("A");
        let b = Element::raw("B");

        let schema = SchemaBuilder::default()
            .add_int_field(a.clone())
            .add_string_field(b.clone(), 9)
            .build();

        let layout = Layout::new(schema);

        let block = tx.append("testfile").unwrap();
        tx.pin(&block).unwrap();
        let record_page = RecordPage::new(&tx, block.clone(), layout).unwrap();
        record_page.format().unwrap();

        let mut rng = rand::rng();

        let mut values_less = HashSet::new();
        let mut values_greater = HashSet::new();

        let mut slot = record_page.insert_after(-1).unwrap();
        while slot >= 0 {
            let n = rng.random::<i32>();
            if n < 25 {
                values_less.insert((n, format!("rec{}", n)));
            } else {
                values_greater.insert((n, format!("rec{}", n)));
            }
            record_page.set_i32(slot, &a, n).unwrap();
            record_page
                .set_string(slot, &b, &format!("rec{}", n))
                .unwrap();
            slot = record_page.insert_after(slot).unwrap();
        }

        let mut slot = record_page.next_after(-1).unwrap();
        while slot >= 0 {
            let a = record_page.get_i32(slot, &a).unwrap();
            let b = record_page.get_string(slot, &b).unwrap();
            if a < 25 {
                assert!(values_less.contains(&(a, b)));
                record_page.delete(slot).unwrap();
            }
            slot = record_page.next_after(slot).unwrap();
        }

        let mut slot = record_page.next_after(-1).unwrap();
        while slot >= 0 {
            let a = record_page.get_i32(slot, &a).unwrap();
            let b = record_page.get_string(slot, &b).unwrap();
            assert!(values_greater.contains(&(a, b)));
            slot = record_page.next_after(slot).unwrap();
        }
        tx.unpin(&block).unwrap();
        tx.commit().unwrap();
    }
}
