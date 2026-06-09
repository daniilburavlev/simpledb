use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use common::DbResult;
use common::error::DbError;
use file::block::BlockId;

use crate::{concurrency::locks::Lock, lock_table::LockTable};

pub struct ConcurrencyMgr {
    lock_table: Arc<LockTable>,
    locks: RwLock<HashMap<BlockId, Lock>>,
}

impl ConcurrencyMgr {
    pub fn new(lock_table: &Arc<LockTable>) -> Self {
        Self {
            lock_table: Arc::clone(lock_table),
            locks: RwLock::new(HashMap::default()),
        }
    }

    pub fn s_lock(&self, block: &BlockId) -> DbResult<()> {
        let mut locks = self.locks.write().map_err(DbError::lock)?;
        if !locks.contains_key(block) {
            self.lock_table.s_lock(block)?;
            locks.insert(block.clone(), Lock::S);
        }
        Ok(())
    }

    pub fn x_lock(&self, block: &BlockId) -> DbResult<()> {
        if !self.has_x_lock(block)? {
            self.s_lock(block)?;
            self.lock_table.x_lock(block)?;
            let mut locks = self.locks.write().map_err(DbError::lock)?;
            locks.insert(block.clone(), Lock::X);
        }
        Ok(())
    }

    pub fn release(&self) -> DbResult<()> {
        let mut write = self.locks.write().map_err(DbError::lock)?;
        for block in write.keys() {
            self.lock_table.unlock(block)?;
        }
        write.clear();
        Ok(())
    }

    fn has_x_lock(&self, block: &BlockId) -> DbResult<bool> {
        let locks = self.locks.read().map_err(DbError::lock)?;
        Ok(matches!(locks.get(block), Some(Lock::X)))
    }
}
