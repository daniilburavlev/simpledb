use common::error::DbError;
use common::{DbResult, locks::lock_with_timeout};
use file::block::BlockId;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

const MAX_TIME: Duration = if cfg!(test) {
    Duration::from_secs(1)
} else {
    Duration::from_secs(10)
};

const SLEEP: Duration = Duration::from_millis(1);

pub struct LockTable {
    locks: Mutex<HashMap<BlockId, i32>>,
}

impl LockTable {
    pub fn s_lock(&self, block: &BlockId) -> DbResult<()> {
        let start = Instant::now();
        loop {
            let mut locks = lock_with_timeout(&self.locks, MAX_TIME)?;
            if !has_x_lock(&locks, block) {
                let val = get_lock_val(&locks, block);
                locks.insert(block.clone(), val + 1);
                return Ok(());
            } else if start.elapsed() >= MAX_TIME {
                return Err(DbError::LockAbort);
            } else {
                drop(locks);
                thread::sleep(SLEEP);
            }
        }
    }

    pub fn x_lock(&self, block: &BlockId) -> DbResult<()> {
        let start = Instant::now();
        loop {
            let mut locks = lock_with_timeout(&self.locks, MAX_TIME)?;
            if !has_other_s_locks(&locks, block) {
                locks.insert(block.clone(), -1);
                return Ok(());
            } else if start.elapsed() >= MAX_TIME {
                return Err(DbError::LockAbort);
            } else {
                drop(locks);
                thread::sleep(SLEEP);
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
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn check_concurrency() {
        let table = Arc::new(LockTable::default());
        let table_clone = Arc::clone(&table);

        thread::spawn(move || {
            let _locks = table_clone.locks.lock().unwrap();
            thread::sleep(Duration::from_secs(12));
        });
        thread::sleep(Duration::from_millis(100));

        if let Err(e) = table.x_lock(&BlockId::new("test", 0)) {
            assert_eq!(e.to_string(), "lock timeout");
        } else {
            panic!("lock timeout check");
        }
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
