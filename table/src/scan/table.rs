use std::sync::{Arc, RwLock};

use common::{DbResult, error::DbError};
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::{
    constant::Constant, field_info::FieldInfo, layout::Layout, record_page::RecordPage, rid::RID,
    scan::Scan,
};

struct TableScanLock {
    tx: Arc<Transaction>,
    layout: Arc<Layout>,
    rp: RecordPage,
    filename: String,
    current_slot: i32,
}

impl TableScanLock {
    fn new(tx: &Arc<Transaction>, table_name: &str, layout: &Arc<Layout>) -> DbResult<Self> {
        let filename = format!("{}.tbl", table_name);
        let rp = if tx.size(&filename)? == 0 {
            let block = tx.append(&filename)?;
            let rp = RecordPage::new(tx, block, layout)?;
            rp.format()?;
            rp
        } else {
            let block = BlockId::new(&filename, 0);
            RecordPage::new(tx, block, layout)?
        };
        let table_scan = Self {
            tx: Arc::clone(tx),
            layout: Arc::clone(layout),
            rp,
            current_slot: -1,
            filename,
        };

        Ok(table_scan)
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

    pub fn get_i32(&self, filename: &str) -> DbResult<i32> {
        self.rp.get_i32(self.current_slot, filename)
    }

    pub fn get_string(&self, filename: &str) -> DbResult<String> {
        self.rp.get_string(self.current_slot, filename)
    }

    pub fn get_val(&self, fieldname: &str) -> DbResult<Constant> {
        let Some(info) = self.layout.schema().info(fieldname)? else {
            return Err(DbError::field_not_exists(fieldname));
        };
        match info {
            FieldInfo::Integer => Ok(Constant::Integer(self.get_i32(fieldname)?)),
            FieldInfo::Varchar(_) => Ok(Constant::Varchar(self.get_string(fieldname)?)),
        }
    }

    pub fn has_field(&self, fieldname: &str) -> DbResult<bool> {
        self.layout.schema().has_field(fieldname)
    }

    pub fn set_i32(&self, field: &str, value: i32) -> DbResult<()> {
        let current_slot = self.current_slot;
        let rp = &self.rp;
        rp.set_i32(current_slot, field, value)
    }

    pub fn set_string(&self, field: &str, value: &str) -> DbResult<()> {
        let current_slot = self.current_slot;
        let rp = &self.rp;
        rp.set_string(current_slot, field, value)
    }

    pub fn set_val(&self, field: &str, value: Constant) -> DbResult<()> {
        match value {
            Constant::Integer(value) => self.set_i32(field, value),
            Constant::Varchar(value) => self.set_string(field, &value),
        }
    }

    pub fn insert(&mut self) -> DbResult<()> {
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
        let rp = RecordPage::new(&self.tx, block, &self.layout)?;
        self.rp = rp;
        Ok(())
    }

    pub fn get_rid(&self) -> RID {
        let rp = &self.rp;
        RID::new(rp.block().num, self.current_slot)
    }

    fn move_to_block(&mut self, num: i32) -> DbResult<()> {
        self.close()?;
        let block = BlockId::new(&self.filename, num);
        self.rp = RecordPage::new(&self.tx, block, &self.layout)?;
        self.current_slot = -1;
        Ok(())
    }

    fn move_to_new_block(&mut self) -> DbResult<()> {
        self.close()?;
        let block = self.tx.append(&self.filename)?;
        self.rp = RecordPage::new(&self.tx, block, &self.layout)?;
        self.rp.format()?;
        self.current_slot = -1;
        Ok(())
    }

    fn at_last_block(&self) -> DbResult<bool> {
        Ok(self.rp.block().num == self.tx.size(&self.filename)? as i32 - 1)
    }
}

pub struct TableScan {
    lock: RwLock<TableScanLock>,
}

impl TableScan {
    pub fn new(tx: &Arc<Transaction>, tablename: &str, layout: &Arc<Layout>) -> DbResult<Self> {
        Ok(Self {
            lock: RwLock::new(TableScanLock::new(tx, tablename, layout)?),
        })
    }
}

impl Scan for TableScan {
    fn before_first(&self) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.before_first()
    }

    fn next(&self) -> DbResult<bool> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.next()
    }

    fn get_i32(&self, field: &str) -> DbResult<i32> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.get_i32(field)
    }

    fn get_string(&self, field: &str) -> DbResult<String> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.get_string(field)
    }

    fn get_val(&self, field: &str) -> DbResult<Constant> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.get_val(field)
    }

    fn has_field(&self, field: &str) -> DbResult<bool> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.has_field(field)
    }

    fn close(&self) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.close()
    }
    fn set_i32(&self, field: &str, value: i32) -> DbResult<()> {
        let write = self.lock.read().map_err(DbError::lock)?;
        write.set_i32(field, value)
    }

    fn set_string(&self, field: &str, value: &str) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.set_string(field, value)
    }

    fn set_val(&self, field: &str, value: Constant) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.set_val(field, value)
    }

    fn insert(&self) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.insert()
    }

    fn delete(&self) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.delete()
    }

    fn move_to_rid(&self, rid: RID) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.move_to_rid(rid)
    }

    fn get_rid(&self) -> DbResult<RID> {
        let read = self.lock.read().map_err(DbError::lock)?;
        Ok(read.get_rid())
    }
}

#[cfg(test)]
mod tests {
    use buffer::mgr::BufferMgr;
    use file::mgr::FileMgr;
    use log::mgr::LogMgr;
    use rand::RngExt;
    use tempfile::tempdir;
    use transaction::{lock_table::LockTable, txnum_generator::TxNumGenerator};

    use crate::schema::Schema;

    use super::*;

    #[test]
    fn table_scan() {
        let dir = tempdir().unwrap();
        let fm = Arc::new(FileMgr::new(dir.path(), 512).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "testlog".to_string()).unwrap());
        let bm = Arc::new(BufferMgr::new(&fm, &lm, 1).unwrap());
        let txnum_generator = TxNumGenerator::default();
        let lock_table = Arc::new(LockTable::default());

        let tx = Arc::new(Transaction::new(&txnum_generator, &fm, &lm, &bm, &lock_table).unwrap());
        let schema = Arc::new(Schema::default());
        schema.add_int_field("A".to_string()).unwrap();
        schema.add_string_field("B".to_string(), 9).unwrap();

        let layout = Arc::new(Layout::new(&schema).unwrap());
        for (field, _) in layout.schema().fields().unwrap() {
            let offset = layout.offset(&field);
            println!("{} has offset {}", field, offset);
        }

        let ts = TableScan::new(&tx, "T", &layout).unwrap();
        println!("Fillins the table with 50 random records");
        ts.before_first().unwrap();
        let mut rng = rand::rng();
        for _ in 0..50 {
            ts.insert().unwrap();
            let n = rng.random::<i32>();
            ts.set_i32("A", n).unwrap();
            ts.set_string("B", &format!("record{}", n)).unwrap();
            println!(
                "inserting into slot {} {{'{}' 'record{}'}}",
                ts.get_rid().unwrap(),
                n,
                n
            );
        }
        println!("Deleting records with A-values < 10.");
        let mut count = 0;
        ts.before_first().unwrap();
        while ts.next().unwrap() {
            let a = ts.get_i32("A").unwrap();
            let b = ts.get_string("B").unwrap();
            if a < 10 {
                count += 1;
                println!("slot {} {{'{}' '{}'}}", ts.get_rid().unwrap(), a, b);
            }
        }
        println!("{} values under 25 were deleted", count);
        println!("Here are the remaining records.");
        ts.before_first().unwrap();
        while ts.next().unwrap() {
            let a = ts.get_i32("A").unwrap();
            let b = ts.get_string("B").unwrap();
            println!("{} slot {{'{}' '{}'}}", ts.get_rid().unwrap(), a, b);
        }
    }
}
