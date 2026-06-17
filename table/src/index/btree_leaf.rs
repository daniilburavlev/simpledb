use std::{
    cmp::Ordering,
    sync::{Arc, RwLock},
};

use common::{DbResult, error::DbError};
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::{
    constant::Constant,
    index::{btree_page::BTreePage, dir_entry::DirEntry},
    layout::Layout,
    rid::RID,
};

pub struct BTreeLeafLock {
    tx: Arc<Transaction>,
    layout: Arc<Layout>,
    key: Constant,
    contents: BTreePage,
    current_slot: i32,
    filename: String,
}

impl BTreeLeafLock {
    pub fn new(
        tx: &Arc<Transaction>,
        block: BlockId,
        layout: &Arc<Layout>,
        key: Constant,
    ) -> DbResult<Self> {
        let filename = block.filename.clone();
        let contents = BTreePage::new(tx, block, layout)?;
        let current_slot = contents.find_slot_before(&key)?;
        Ok(Self {
            tx: Arc::clone(tx),
            layout: Arc::clone(layout),
            filename,
            current_slot,
            key,
            contents,
        })
    }

    pub fn close(&self) -> DbResult<()> {
        self.contents.close()
    }

    pub fn next(&mut self) -> DbResult<bool> {
        self.current_slot += 1;
        if self.current_slot >= self.contents.get_num_recs()? {
            self.try_overflow()
        } else if self.contents.get_data_val(self.current_slot)? == self.key {
            Ok(true)
        } else {
            self.try_overflow()
        }
    }

    pub fn get_data_rid(&self) -> DbResult<RID> {
        self.contents.get_data_rid(self.current_slot)
    }

    pub fn insert(&mut self, rid: RID) -> DbResult<Option<DirEntry>> {
        if self.contents.get_flag()? >= 0
            && self.contents.get_data_val(0)?.cmp(&self.key) == Ordering::Greater
        {
            let first_val = self.contents.get_data_val(0)?;
            let new_block = self.contents.split(0, self.contents.get_flag()?)?;
            self.contents.set_flag(-1)?;
            self.current_slot = 0;
            self.contents
                .insert_leaf(self.current_slot, self.key.clone(), rid)?;
            return Ok(Some(DirEntry::new(first_val, new_block.num)));
        }
        self.current_slot += 1;
        self.contents
            .insert_leaf(self.current_slot, self.key.clone(), rid)?;
        if !self.contents.is_full()? {
            return Ok(None);
        }
        let first_key = self.contents.get_data_val(0)?;
        let last_key = self
            .contents
            .get_data_val(self.contents.get_num_recs()? - 1)?;
        if last_key == first_key {
            let new_block = self.contents.split(1, self.contents.get_flag()?)?;
            self.contents.set_flag(new_block.num)?;
            Ok(None)
        } else {
            let mut split_pos = self.contents.get_num_recs()? / 2;
            let mut split_key = self.contents.get_data_val(split_pos)?;
            if split_key == first_key {
                while self.contents.get_data_val(split_pos)? == split_key {
                    split_pos += 1;
                }
                split_key = self.contents.get_data_val(split_pos)?;
            } else {
                while self.contents.get_data_val(split_pos - 1)? == split_key {
                    split_pos -= 1;
                }
            }
            let new_block = self.contents.split(split_pos, -1)?;
            Ok(Some(DirEntry::new(split_key, new_block.num)))
        }
    }

    pub fn try_overflow(&mut self) -> DbResult<bool> {
        let first_key = self.contents.get_data_val(0)?;
        let flag = self.contents.get_flag()?;
        if self.key != first_key || flag < 0 {
            return Ok(false);
        }
        self.contents.close()?;
        let next_block = BlockId::new(&self.filename, flag);
        self.contents = BTreePage::new(&self.tx, next_block, &self.layout)?;
        self.current_slot = 0;
        Ok(true)
    }

    fn delete(&mut self, rid: RID) -> DbResult<()> {
        while self.next()? {
            if self.get_data_rid()? == rid {
                self.contents.delete(self.current_slot)?;
                return Ok(());
            }
        }
        Ok(())
    }
}

pub struct BTreeLeaf {
    lock: RwLock<BTreeLeafLock>,
}

impl BTreeLeaf {
    pub fn new(
        tx: &Arc<Transaction>,
        block: BlockId,
        layout: &Arc<Layout>,
        key: Constant,
    ) -> DbResult<Self> {
        Ok(Self {
            lock: RwLock::new(BTreeLeafLock::new(tx, block, layout, key)?),
        })
    }

    pub fn insert(&self, rid: RID) -> DbResult<Option<DirEntry>> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.insert(rid)
    }

    pub fn next(&self) -> DbResult<bool> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.next()
    }

    pub fn get_data_rid(&self) -> DbResult<RID> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.get_data_rid()
    }

    pub fn close(&self) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.close()
    }

    pub fn delete(&self, rid: RID) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.delete(rid)
    }
}
