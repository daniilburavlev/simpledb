use std::{
    cmp::Ordering,
    sync::{Arc, RwLock},
};

use common::{DbResult, error::DbError};
use file::{block::BlockId, page::I32_SIZE};
use transaction::transaction::Transaction;

use crate::{constant::Constant, field_info::FieldInfo, layout::Layout, rid::RID};

pub const VALUE: &str = "value";
pub const BLOCK: &str = "block";
pub const ID: &str = "id";

struct BTreePageLock {
    tx: Arc<Transaction>,
    current_block: Option<BlockId>,
    layout: Arc<Layout>,
}

impl BTreePageLock {
    fn new(tx: &Arc<Transaction>, block: BlockId, layout: &Arc<Layout>) -> DbResult<Self> {
        tx.pin(&block)?;
        Ok(Self {
            tx: Arc::clone(tx),
            current_block: Some(block),
            layout: Arc::clone(layout),
        })
    }

    fn find_slot_before(&self, key: &Constant) -> DbResult<i32> {
        let mut slot = 0;
        while slot < self.get_num_recs()? && self.get_data_val(slot)?.cmp(key) == Ordering::Less {
            slot += 1
        }
        Ok(slot - 1)
    }

    fn close(&mut self) -> DbResult<()> {
        if let Some(block) = &self.current_block {
            self.tx.unpin(block)?;
        }
        self.current_block = None;
        Ok(())
    }

    fn is_full(&self) -> DbResult<bool> {
        Ok(self.slotpos(self.get_num_recs()? + 1) >= self.tx.block_size())
    }

    fn split(&self, splitpos: i32, flag: i32) -> DbResult<BlockId> {
        let new_block = self.append_new(flag)?;
        let mut page = BTreePageLock::new(&self.tx, new_block.clone(), &self.layout)?;
        self.transfer_records(splitpos, &page)?;
        page.set_flag(flag)?;
        page.close()?;
        Ok(new_block)
    }

    fn get_data_val(&self, slot: i32) -> DbResult<Constant> {
        self.get_val(slot, VALUE)
    }

    fn get_flag(&self) -> DbResult<i32> {
        if let Some(block) = &self.current_block {
            self.tx.get_i32(block, 0)
        } else {
            Err(DbError::other("cannot access index flag"))
        }
    }

    fn set_flag(&self, flag: i32) -> DbResult<()> {
        if let Some(block) = &self.current_block {
            self.tx.set_i32(block, 0, flag, true)
        } else {
            Err(DbError::other("cannot access index flag"))
        }
    }

    fn append_new(&self, flag: i32) -> DbResult<BlockId> {
        let Some(current_block) = &self.current_block else {
            return Err(DbError::other("cannot access index flag"));
        };
        let block = self.tx.append(&current_block.filename)?;
        self.tx.pin(&block)?;
        self.format(&block, flag)?;
        Ok(block)
    }

    fn format(&self, block: &BlockId, flag: i32) -> DbResult<()> {
        self.tx.set_i32(block, 0, flag, false)?;
        self.tx.set_i32(block, I32_SIZE, 0, false)?;
        let recsize = self.layout.slotsize();
        let mut pos = 2 * I32_SIZE as i32;
        while pos + recsize <= self.tx.block_size() {
            self.make_default_record(block, pos)?;
            pos += recsize;
        }
        Ok(())
    }

    fn make_default_record(&self, block: &BlockId, pos: i32) -> DbResult<()> {
        for (field, info) in self.layout.schema().fields()? {
            let offset = self.layout.offset(&field);
            let offset = pos as usize + offset as usize;
            match info {
                FieldInfo::Integer => self.tx.set_i32(block, offset, 0, false)?,
                FieldInfo::Varchar(_) => self.tx.set_string(block, offset, "", false)?,
            }
        }
        Ok(())
    }

    fn get_child_num(&self, slot: i32) -> DbResult<i32> {
        self.get_i32(slot, BLOCK)
    }

    fn insert_dir(&self, slot: i32, value: Constant, blocknum: i32) -> DbResult<()> {
        self.insert(slot)?;
        self.set_val(slot, VALUE, value)?;
        self.set_i32(slot, BLOCK, blocknum)?;
        Ok(())
    }

    fn get_data_rid(&self, slot: i32) -> DbResult<RID> {
        let block_num = self.get_i32(slot, BLOCK)?;
        let slot = self.get_i32(slot, ID)?;
        Ok(RID::new(block_num, slot))
    }

    fn insert_leaf(&self, slot: i32, value: Constant, rid: RID) -> DbResult<()> {
        self.insert(slot)?;
        self.set_val(slot, VALUE, value)?;
        self.set_i32(slot, BLOCK, rid.block_num())?;
        self.set_i32(slot, ID, rid.slot())?;
        Ok(())
    }

    fn delete(&self, slot: i32) -> DbResult<()> {
        let start = slot as usize + 1;
        let end = self.get_num_recs()? as usize;
        for i in start..end {
            let i = i as i32;
            self.copy_record(i, i - 1)?;
        }
        self.set_num_recs(self.get_num_recs()? - 1)?;
        Ok(())
    }

    fn get_num_recs(&self) -> DbResult<i32> {
        if let Some(block) = &self.current_block {
            self.tx.get_i32(block, 4)
        } else {
            Err(DbError::other("cannot get block"))
        }
    }

    fn get_i32(&self, slot: i32, field: &str) -> DbResult<i32> {
        let pos = self.field_pos(slot, field);
        if let Some(block) = &self.current_block {
            self.tx.get_i32(block, pos as usize)
        } else {
            Err(DbError::other("cannot get integer"))
        }
    }

    fn get_string(&self, slot: i32, field: &str) -> DbResult<String> {
        let pos = self.field_pos(slot, field);
        if let Some(block) = &self.current_block {
            self.tx.get_string(block, pos as usize)
        } else {
            Err(DbError::other("cannot get string value"))
        }
    }

    fn get_val(&self, slot: i32, field: &str) -> DbResult<Constant> {
        let Some(info) = self.layout.schema().info(field)? else {
            return Err(DbError::FieldNotExists(field.to_string()));
        };
        match info {
            FieldInfo::Integer => Ok(Constant::Integer(self.get_i32(slot, field)?)),
            FieldInfo::Varchar(_) => Ok(Constant::Varchar(self.get_string(slot, field)?)),
        }
    }

    fn set_i32(&self, slot: i32, field: &str, value: i32) -> DbResult<()> {
        let pos = self.field_pos(slot, field);
        if let Some(block) = &self.current_block {
            self.tx.set_i32(block, pos as usize, value, true)
        } else {
            Err(DbError::other("cannot set integer value"))
        }
    }

    fn set_string(&self, slot: i32, field: &str, value: &str) -> DbResult<()> {
        let pos = self.field_pos(slot, field);
        if let Some(block) = &self.current_block {
            self.tx.set_string(block, pos as usize, value, true)
        } else {
            Err(DbError::other("cannot set integer value"))
        }
    }

    fn set_val(&self, slot: i32, field: &str, value: Constant) -> DbResult<()> {
        match self.layout.schema().info(field)? {
            Some(FieldInfo::Integer) => self.set_i32(slot, field, value.as_i32()?),
            Some(FieldInfo::Varchar(_)) => self.set_string(slot, field, value.as_str()?),
            None => Err(DbError::FieldNotExists(field.to_string())),
        }
    }

    fn set_num_recs(&self, n: i32) -> DbResult<()> {
        let Some(block) = &self.current_block else {
            return Err(DbError::other("cannot set num recs"));
        };
        self.tx.set_i32(block, I32_SIZE, n, true)
    }

    fn insert(&self, slot: i32) -> DbResult<()> {
        let num_recs = self.get_num_recs()?;
        for i in (slot + 1..=num_recs).rev() {
            self.copy_record(i - 1, i)?;
        }
        self.set_num_recs(num_recs + 1)
    }

    fn copy_record(&self, from: i32, to: i32) -> DbResult<()> {
        let schema = self.layout.schema();
        for (field, _) in schema.fields()? {
            let value = self.get_val(from, &field)?;
            self.set_val(to, &field, value)?;
        }
        Ok(())
    }

    fn transfer_records(&self, slot: i32, dest: &Self) -> DbResult<()> {
        let mut dest_slot = 0;
        while slot < self.get_num_recs()? {
            dest.insert(dest_slot)?;
            let schema = self.layout.schema();
            for (field, _) in schema.fields()? {
                dest.set_val(dest_slot, &field, self.get_val(slot, &field)?)?;
            }
            self.delete(slot)?;
            dest_slot += 1;
        }
        Ok(())
    }

    fn field_pos(&self, slot: i32, field: &str) -> i32 {
        let offset = self.layout.offset(field);
        self.slotpos(slot) + offset
    }

    fn slotpos(&self, slot: i32) -> i32 {
        let slotsize = self.layout.slotsize();
        4 + 4 + (slot * slotsize)
    }
}

pub struct BTreePage {
    lock: RwLock<BTreePageLock>,
}

impl BTreePage {
    pub fn new(tx: &Arc<Transaction>, block: BlockId, layout: &Arc<Layout>) -> DbResult<Self> {
        Ok(Self {
            lock: RwLock::new(BTreePageLock::new(tx, block, layout)?),
        })
    }

    pub fn find_slot_before(&self, key: &Constant) -> DbResult<i32> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.find_slot_before(key)
    }

    pub fn close(&self) -> DbResult<()> {
        let mut read = self.lock.write().map_err(DbError::lock)?;
        read.close()
    }

    pub fn is_full(&self) -> DbResult<bool> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.is_full()
    }

    pub fn get_num_recs(&self) -> DbResult<i32> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.get_num_recs()
    }

    pub fn get_data_val(&self, slot: i32) -> DbResult<Constant> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.get_data_val(slot)
    }

    pub fn get_data_rid(&self, slot: i32) -> DbResult<RID> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.get_data_rid(slot)
    }

    pub fn get_flag(&self) -> DbResult<i32> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.get_flag()
    }

    pub fn split(&self, pos: i32, flag: i32) -> DbResult<BlockId> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.split(pos, flag)
    }

    pub fn set_flag(&self, flag: i32) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.set_flag(flag)
    }

    pub fn insert_leaf(&self, slot: i32, value: Constant, rid: RID) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.insert_leaf(slot, value, rid)
    }

    pub fn insert_dir(&self, slot: i32, value: Constant, blocknum: i32) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.insert_dir(slot, value, blocknum)
    }

    pub fn get_child_num(&self, slot: i32) -> DbResult<i32> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.get_child_num(slot)
    }

    pub fn format(&self, block: &BlockId, flag: i32) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.format(block, flag)
    }

    pub fn delete(&self, slot: i32) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.delete(slot)
    }
}
