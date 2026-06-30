use std::sync::{Arc, RwLock};

use common::{DbResult, error::DbError};
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::{
    constant::Constant,
    index::{btree_page::BTreePage, dir_entry::DirEntry},
    layout::Layout,
};

pub struct BTreeDirLock {
    tx: Arc<Transaction>,
    layout: Arc<Layout>,
    contents: BTreePage,
    filename: String,
}

impl BTreeDirLock {
    pub fn new(tx: &Arc<Transaction>, block: BlockId, layout: &Arc<Layout>) -> DbResult<Self> {
        let filename = block.filename.clone();
        Ok(Self {
            tx: Arc::clone(tx),
            layout: Arc::clone(layout),
            contents: BTreePage::new(tx, block, layout)?,
            filename,
        })
    }

    pub fn close(&self) -> DbResult<()> {
        self.contents.close()
    }

    pub fn search(&mut self, key: &Constant) -> DbResult<i32> {
        let mut child_block = self.find_child_block(key)?;
        while self.contents.get_flag()? > 0 {
            self.contents.close()?;
            self.contents = BTreePage::new(&self.tx, child_block.clone(), &self.layout)?;
            child_block = self.find_child_block(key)?;
        }
        Ok(child_block.num)
    }

    pub fn make_new_root(&self, e: DirEntry) -> DbResult<()> {
        let first_val = self.contents.get_data_val(0)?;
        let level = self.contents.get_flag()?;
        let new_block = self.contents.split(0, level)?;
        let old_root = DirEntry::new(first_val, new_block.num);
        self.insert_entry(old_root)?;
        self.insert_entry(e)?;
        self.contents.set_flag(level + 1)?;
        Ok(())
    }

    pub fn insert(&self, e: DirEntry) -> DbResult<Option<DirEntry>> {
        if self.contents.get_flag()? == 0 {
            return self.insert_entry(e);
        }
        let child_block = self.find_child_block(&e.value)?;
        let child = BTreeDirLock::new(&self.tx, child_block, &self.layout)?;
        let entry = child.insert(e)?;
        child.close()?;
        if let Some(entry) = entry {
            self.insert_entry(entry)
        } else {
            Ok(None)
        }
    }

    fn insert_entry(&self, e: DirEntry) -> DbResult<Option<DirEntry>> {
        let new_slot = 1 + self.contents.find_slot_before(&e.value)?;
        self.contents.insert_dir(new_slot, e.value, e.block_num)?;
        if !self.contents.is_full()? {
            return Ok(None);
        }
        let level = self.contents.get_flag()?;
        let split_pos = self.contents.get_num_recs()? / 2;
        let split_val = self.contents.get_data_val(split_pos)?;
        let new_block = self.contents.split(split_pos, level)?;
        Ok(Some(DirEntry::new(split_val, new_block.num)))
    }

    fn find_child_block(&self, key: &Constant) -> DbResult<BlockId> {
        let mut slot = self.contents.find_slot_before(key)?;
        if self.contents.get_data_val(slot + 1)? == *key {
            slot += 1;
        }
        let block_num = self.contents.get_child_num(slot)?;
        Ok(BlockId::new(&self.filename, block_num))
    }
}

pub struct BTreeDir {
    lock: RwLock<BTreeDirLock>,
}

impl BTreeDir {
    pub fn new(tx: &Arc<Transaction>, block: BlockId, layout: &Arc<Layout>) -> DbResult<Self> {
        Ok(Self {
            lock: RwLock::new(BTreeDirLock::new(tx, block, layout)?),
        })
    }

    pub fn close(&self) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.close()
    }

    pub fn search(&self, key: &Constant) -> DbResult<i32> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.search(key)
    }

    pub fn make_new_root(&self, e: DirEntry) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.make_new_root(e)
    }

    pub fn insert(&self, e: DirEntry) -> DbResult<Option<DirEntry>> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.insert(e)
    }
}
