use std::cell::RefCell;
use std::sync::Arc;

use common::{DbResult, error::DbError};
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::element::Element;
use crate::schema::Schema;
use crate::{
    field_info::FieldInfo, layout::Layout, record_page::RecordPage, rid::RID, scan::Scan,
    value::Value,
};

struct TableScanInner {
    tx: Arc<Transaction>,
    layout: Layout,
    rp: RecordPage,
    filename: String,
    current_slot: i32,
}

impl TableScanInner {
    fn new(tx: &Arc<Transaction>, table_name: &str, layout: Layout) -> DbResult<Self> {
        let filename = format!("{}.tbl", table_name);
        let rp = if tx.size(&filename)? == 0 {
            let block = tx.append(&filename)?;
            let rp = RecordPage::new(tx, block, layout.clone())?;
            rp.format()?;
            rp
        } else {
            let block = BlockId::new(&filename, 0);
            RecordPage::new(tx, block, layout.clone())?
        };
        Ok(Self {
            tx: Arc::clone(tx),
            layout,
            rp,
            current_slot: -1,
            filename,
        })
    }

    fn close(&self) -> DbResult<()> {
        self.tx.unpin(&self.rp.block())
    }

    pub fn before_first(&mut self) -> DbResult<()> {
        self.move_to_block(0)
    }

    pub fn next(&mut self) -> DbResult<bool> {
        self.current_slot = self.rp.next_after(self.current_slot)?;
        while self.current_slot < 0 {
            if self.at_last_block()? {
                return Ok(false);
            }
            self.move_to_block(self.rp.block().num + 1)?;
            self.current_slot = self.rp.next_after(self.current_slot)?;
        }
        Ok(true)
    }

    pub fn get_i32(&self, filename: &Element) -> DbResult<i32> {
        self.rp.get_i32(self.current_slot, filename)
    }

    pub fn get_string(&self, filename: &Element) -> DbResult<String> {
        self.rp.get_string(self.current_slot, filename)
    }

    pub fn get_val(&self, fieldname: &Element) -> DbResult<Value> {
        let Some(info) = self.layout.schema().info(fieldname) else {
            return Err(DbError::FieldNotExists(fieldname.to_string()));
        };
        match info {
            FieldInfo::Integer => Ok(Value::Integer(self.get_i32(fieldname)?)),
            FieldInfo::Varchar(_) => Ok(Value::Varchar(self.get_string(fieldname)?)),
        }
    }

    pub fn has_field(&self, fieldname: &Element) -> bool {
        self.layout.schema().has_field(fieldname)
    }

    pub fn set_i32(&self, field: &Element, value: i32) -> DbResult<()> {
        let current_slot = self.current_slot;
        let rp = &self.rp;
        rp.set_i32(current_slot, field, value)
    }

    pub fn set_string(&self, field: &Element, value: &str) -> DbResult<()> {
        let current_slot = self.current_slot;
        let rp = &self.rp;
        rp.set_string(current_slot, field, value)
    }

    pub fn set_val(&self, field: &Element, value: Value) -> DbResult<()> {
        match value {
            Value::Integer(value) => self.set_i32(field, value),
            Value::Varchar(value) => self.set_string(field, &value),
        }
    }

    pub fn insert(&mut self) -> DbResult<()> {
        let last_block = self.tx.size(&self.filename)? as i32 - 1;
        if self.rp.block().num != last_block {
            self.move_to_block(last_block)?;
        }
        self.current_slot = self.rp.insert_after(self.current_slot)?;
        while self.current_slot < 0 {
            if self.at_last_block()? {
                self.move_to_new_block()?;
            } else {
                self.move_to_block(self.rp.block().num + 1)?;
            }
            self.current_slot = self.rp.insert_after(self.current_slot)?;
        }
        Ok(())
    }

    pub fn delete(&self) -> DbResult<()> {
        self.rp.delete(self.current_slot)
    }

    pub fn move_to_rid(&mut self, rid: RID) -> DbResult<()> {
        self.close()?;
        let block = BlockId::new(&self.filename, rid.block_num());
        let rp = RecordPage::new(&self.tx, block, self.layout.clone())?;
        self.rp = rp;
        self.current_slot = rid.slot();
        Ok(())
    }

    pub fn get_rid(&self) -> RID {
        let rp = &self.rp;
        RID::new(rp.block().num, self.current_slot)
    }

    fn move_to_block(&mut self, num: i32) -> DbResult<()> {
        self.close()?;
        let block = BlockId::new(&self.filename, num);
        self.rp = RecordPage::new(&self.tx, block, self.layout.clone())?;
        self.current_slot = -1;
        Ok(())
    }

    fn move_to_new_block(&mut self) -> DbResult<()> {
        self.close()?;
        let block = self.tx.append(&self.filename)?;
        self.rp = RecordPage::new(&self.tx, block, self.layout.clone())?;
        self.rp.format()?;
        self.current_slot = -1;
        Ok(())
    }

    fn at_last_block(&self) -> DbResult<bool> {
        Ok(self.rp.block().num == self.tx.size(&self.filename)? as i32 - 1)
    }

    fn schema(&self) -> DbResult<Schema> {
        Ok(self.layout.schema().clone())
    }
}

pub struct TableScan {
    lock: RefCell<TableScanInner>,
}

impl TableScan {
    pub fn new(tx: &Arc<Transaction>, tablename: &str, layout: Layout) -> DbResult<Self> {
        Ok(Self {
            lock: RefCell::new(TableScanInner::new(tx, tablename, layout)?),
        })
    }
}

impl Scan for TableScan {
    fn before_first(&self) -> DbResult<()> {
        let mut write = self.lock.borrow_mut();
        write.before_first()
    }

    fn next(&self) -> DbResult<bool> {
        let mut write = self.lock.borrow_mut();
        write.next()
    }

    fn get_i32(&self, field: &Element) -> DbResult<i32> {
        let read = self.lock.borrow();
        read.get_i32(field)
    }

    fn get_string(&self, field: &Element) -> DbResult<String> {
        let read = self.lock.borrow();
        read.get_string(field)
    }

    fn get_val(&self, field: &Element) -> DbResult<Value> {
        let read = self.lock.borrow();
        read.get_val(field)
    }

    fn has_field(&self, field: &Element) -> DbResult<bool> {
        let read = self.lock.borrow();
        Ok(read.has_field(field))
    }

    fn close(&self) -> DbResult<()> {
        let read = self.lock.borrow();
        read.close()
    }

    fn set_i32(&self, field: &Element, value: i32) -> DbResult<()> {
        let read = self.lock.borrow();
        read.set_i32(field, value)
    }

    fn set_string(&self, field: &Element, value: &str) -> DbResult<()> {
        let read = self.lock.borrow();
        read.set_string(field, value)
    }

    fn set_val(&self, field: &Element, value: Value) -> DbResult<()> {
        let read = self.lock.borrow();
        read.set_val(field, value)
    }

    fn insert(&self) -> DbResult<()> {
        let mut write = self.lock.borrow_mut();
        write.insert()
    }

    fn delete(&self) -> DbResult<()> {
        let read = self.lock.borrow();
        read.delete()
    }

    fn move_to_rid(&self, rid: RID) -> DbResult<()> {
        let mut write = self.lock.borrow_mut();
        write.move_to_rid(rid)
    }

    fn get_rid(&self) -> DbResult<RID> {
        let read = self.lock.borrow();
        Ok(read.get_rid())
    }

    fn schema(&self) -> DbResult<Schema> {
        let read = self.lock.borrow();
        read.schema()
    }
}

#[cfg(test)]
mod tests {
    use crate::schema::SchemaBuilder;
    use buffer::mgr::BufferMgr;
    use file::mgr::FileMgr;
    use log::mgr::LogMgr;
    use tempfile::tempdir;
    use transaction::lock_table::LockTable;

    use super::*;

    #[test]
    fn table_scan() {
        let dir = tempdir().unwrap();
        let fm = Arc::new(FileMgr::new(dir.path(), 512).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "testlog".to_string()).unwrap());
        let bm = Arc::new(BufferMgr::new(&fm, &lm, 1).unwrap());
        let lock_table = Arc::new(LockTable::default());

        let tx = Arc::new(Transaction::new(&fm, &lm, &bm, &lock_table).unwrap());
        let schema = SchemaBuilder::new(Element::raw("test"))
            .add_int_field(Element::raw("A"))
            .add_string_field(Element::raw("B"), 9)
            .build();

        let layout = Layout::new(schema);
        let offset_a = layout.offset(&Element::raw("A"));
        let offset_b = layout.offset(&Element::raw("B"));
        assert_ne!(offset_a, offset_b, "fields must occupy distinct offsets");

        // Fill the table with 50 records carrying known A-values 0..50.
        let ts = TableScan::new(&tx, "T", layout).unwrap();
        ts.before_first().unwrap();
        for n in 0..50 {
            ts.insert().unwrap();
            ts.set_i32(&Element::raw("A"), n).unwrap();
            ts.set_string(&Element::raw("B"), &format!("record{}", n))
                .unwrap();
        }

        // Delete every record whose A-value is below 10 (exactly 0..10).
        let mut deleted = 0;
        ts.before_first().unwrap();
        while ts.next().unwrap() {
            let a = ts.get_i32(&Element::raw("A")).unwrap();
            if a < 10 {
                ts.delete().unwrap();
                deleted += 1;
            }
        }
        assert_eq!(deleted, 10, "records with A < 10 should be deleted");

        // The remaining records are exactly the 40 with A-values 10..50.
        let mut remaining = 0;
        ts.before_first().unwrap();
        while ts.next().unwrap() {
            let a = ts.get_i32(&Element::raw("A")).unwrap();
            let b = ts.get_string(&Element::raw("B")).unwrap();
            assert!(a >= 10, "deleted records should not survive");
            assert_eq!(b, format!("record{}", a), "A and B must stay paired");
            remaining += 1;
        }
        assert_eq!(remaining, 40, "40 records should remain after deletion");
    }
}
