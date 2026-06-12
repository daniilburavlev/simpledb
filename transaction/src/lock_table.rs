use common::error::DbError;
use common::{DbResult, locks::lock_with_timeout};
use file::block::BlockId;
use std::collections::HashMap;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

const MAX_TIME: Duration = if cfg!(test) {
    Duration::from_secs(1)
} else {
    Duration::from_secs(10)
};

pub struct LockTable {
    locks: Mutex<HashMap<BlockId, i32>>,
    cond: Condvar,
}

impl LockTable {
    pub fn s_lock(&self, block: &BlockId) -> DbResult<()> {
        let start = Instant::now();
        let mut locks = lock_with_timeout(&self.locks, MAX_TIME)?;
        loop {
            if !has_x_lock(&locks, block) {
                let val = get_lock_val(&locks, block);
                locks.insert(block.clone(), val + 1);
                return Ok(());
            }
            if start.elapsed() >= MAX_TIME {
                return Err(DbError::LockAbort);
            }
            let (new_guard, timeout) = self
                .cond
                .wait_timeout(locks, MAX_TIME - start.elapsed())
                .map_err(DbError::lock)?;
            locks = new_guard;
            if timeout.timed_out() {
                return Err(DbError::LockAbort);
            }
        }
    }

    pub fn x_lock(&self, block: &BlockId) -> DbResult<()> {
        let start = Instant::now();
        let mut locks = lock_with_timeout(&self.locks, MAX_TIME)?;
        loop {
            if !has_other_s_locks(&locks, block) {
                locks.insert(block.clone(), -1);
                return Ok(());
            }
            if start.elapsed() >= MAX_TIME {
                return Err(DbError::LockAbort);
            }
            let (new_guard, timeout) = self
                .cond
                .wait_timeout(locks, MAX_TIME - start.elapsed())
                .map_err(DbError::lock)?;
            locks = new_guard;
            if timeout.timed_out() {
                return Err(DbError::LockAbort);
            }
        }
    }

    pub fn unlock(&self, block: &BlockId) -> DbResult<()> {
        let mut locks = self.locks.lock().map_err(DbError::lock)?;
        let val = get_lock_val(&locks, block);
        if val > 1 {
            locks.insert(block.clone(), val - 1);
        } else {
            locks.remove(block);
        }
        self.cond.notify_all();
        Ok(())
    }
}

fn has_x_lock(locks: &MutexGuard<'_, HashMap<BlockId, i32>>, block: &BlockId) -> bool {
    get_lock_val(locks, block) < 0
}

fn has_other_s_locks(locks: &MutexGuard<'_, HashMap<BlockId, i32>>, block: &BlockId) -> bool {
    get_lock_val(locks, block) > 1
}

fn get_lock_val(locks: &MutexGuard<'_, HashMap<BlockId, i32>>, block: &BlockId) -> i32 {
    if let Some(value) = locks.get(block) {
        *value
    } else {
        0
    }
}

impl Default for LockTable {
    fn default() -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
            cond: Condvar::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::*;

    #[test]
    fn x_lock_wakes_on_unlock() {
        let table = Arc::new(LockTable::default());
        let block = BlockId::new("test", 0);

        // two s_locks make val > 1, so x_lock must wait
        table.s_lock(&block).unwrap();
        table.s_lock(&block).unwrap();

        let t = Arc::clone(&table);
        let b = block.clone();
        let handle = thread::spawn(move || t.x_lock(&b).unwrap());

        thread::sleep(Duration::from_millis(50));
        // releasing both s_locks should wake the x_lock waiter
        table.unlock(&block).unwrap();
        table.unlock(&block).unwrap();

        handle.join().unwrap();
    }

    #[test]
    fn x_lock() {
        let table = Arc::new(LockTable::default());
        let table_clone = Arc::clone(&table);

        let block = BlockId::new("test", 0);
        let block_clone = block.clone();

        thread::spawn(move || {
            table_clone.s_lock(&block_clone).unwrap();
            thread::sleep(Duration::from_secs(10));
        });
        thread::sleep(Duration::from_millis(100));

        table.s_lock(&block).unwrap();
        if let Err(e) = table.x_lock(&BlockId::new("test", 0)) {
            assert_eq!(e.to_string(), "lock abort");
        } else {
            panic!("lock timeout check");
        }
    }

    #[test]
    fn s_lock() {
        let table = Arc::new(LockTable::default());
        let table_clone = Arc::clone(&table);

        let block = BlockId::new("test", 0);
        let block_clone = block.clone();

        thread::spawn(move || {
            table_clone.x_lock(&block_clone).unwrap();
            thread::sleep(Duration::from_secs(10));
        });
        thread::sleep(Duration::from_millis(100));

        if let Err(e) = table.s_lock(&BlockId::new("test", 0)) {
            assert_eq!(e.to_string(), "lock abort");
        } else {
            panic!("lock timeout check");
        }
    }

    #[test]
    fn unlock() {
        let table = Arc::new(LockTable::default());
        let table_clone = Arc::clone(&table);

        let block = BlockId::new("test", 0);
        let block_clone = block.clone();

        thread::spawn(move || {
            table_clone.x_lock(&block_clone).unwrap();
            table_clone.unlock(&block_clone).unwrap();
        });
        thread::sleep(Duration::from_millis(100));

        table.s_lock(&block).unwrap();
    }
}
