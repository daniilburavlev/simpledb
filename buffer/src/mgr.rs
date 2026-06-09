use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use common::locks::lock_with_timeout;
use common::{DbResult, error::DbError};
use file::{block::BlockId, mgr::FileMgr};
use log::mgr::LogMgr;

use crate::buffer::Buffer;

const MAX_WAIT: Duration = Duration::from_secs(10);
const SLEEP: Duration = Duration::from_millis(1);

struct BufferPool {
    pool: Vec<Buffer>,
    num_available: AtomicU32,
}

impl BufferPool {
    fn new(fm: &Arc<FileMgr>, lm: &Arc<LogMgr>, num_buffs: usize) -> DbResult<Self> {
        let mut pool = Vec::with_capacity(num_buffs);
        for _ in 0..num_buffs {
            pool.push(Buffer::new(fm, lm)?);
        }
        Ok(Self {
            pool,
            num_available: AtomicU32::new(num_buffs as u32),
        })
    }

    fn available(&self) -> u32 {
        self.num_available.load(Ordering::SeqCst)
    }

    fn flush_all(&self, txnum: i32) -> DbResult<()> {
        for buff in self.pool.iter() {
            if buff.modifying_tx()? == txnum {
                buff.flush()?;
            }
        }
        Ok(())
    }

    fn unpin(&self, buffer: Buffer) -> DbResult<()> {
        buffer.unpin()?;
        if !buffer.is_pinned()? {
            self.num_available.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }

    fn try_to_pin(&self, block: &BlockId) -> DbResult<Option<Buffer>> {
        let buffer = if let Some(buffer) = self.find_existing_buffer(block)? {
            buffer
        } else {
            let Some(buffer) = self.choose_unpinned_buffer()? else {
                return Ok(None);
            };
            buffer.assign_to_block(block)?;
            buffer
        };
        if !buffer.is_pinned()? {
            self.num_available.fetch_sub(1, Ordering::SeqCst);
        }
        buffer.pin()?;
        Ok(Some(buffer))
    }

    fn find_existing_buffer(&self, block: &BlockId) -> DbResult<Option<Buffer>> {
        for buff in &self.pool {
            if let Some(b) = buff.block()?
                && &b == block
            {
                return Ok(Some(buff.clone()));
            }
        }
        Ok(None)
    }

    fn choose_unpinned_buffer(&self) -> DbResult<Option<Buffer>> {
        for buffer in &self.pool {
            if !buffer.is_pinned()? {
                return Ok(Some(buffer.clone()));
            }
        }
        Ok(None)
    }
}

pub struct BufferMgr {
    pool: Mutex<BufferPool>,
}

impl BufferMgr {
    pub fn new(fm: &Arc<FileMgr>, lm: &Arc<LogMgr>, num_buffs: usize) -> DbResult<Self> {
        let pool = Mutex::new(BufferPool::new(fm, lm, num_buffs)?);
        Ok(Self { pool })
    }

    pub fn available(&self) -> DbResult<u32> {
        let lock = self.pool.lock().map_err(DbError::lock)?;
        Ok(lock.available())
    }

    pub fn flush_all(&self, txnum: i32) -> DbResult<()> {
        let lock = self.pool.lock().map_err(DbError::lock)?;
        lock.flush_all(txnum)?;
        Ok(())
    }

    pub fn pin(&self, block: &BlockId) -> DbResult<Buffer> {
        let start = Instant::now();
        loop {
            let lock = lock_with_timeout(&self.pool, MAX_WAIT)?;
            match lock.try_to_pin(block)? {
                Some(buffer) => {
                    return Ok(buffer);
                }
                None if start.elapsed() >= MAX_WAIT => return Err(DbError::BufferAbort),
                None => {
                    drop(lock);
                    thread::sleep(SLEEP)
                }
            };
        }
    }

    pub fn unpin(&self, buffer: Buffer) -> DbResult<()> {
        let lock = self.pool.lock().map_err(DbError::lock)?;
        lock.unpin(buffer)
    }
}

#[cfg(test)]
mod tests {
    use file::block::BlockId;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn buffer() {
        let dir = tempdir().unwrap();
        let fm = Arc::new(FileMgr::new(dir.path(), 512).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "wal".to_string()).unwrap());

        let bm = BufferMgr::new(&fm, &lm, 3).unwrap();
        let buff1 = bm.pin(&BlockId::new("testfile", 1)).unwrap();

        let offset = 80;

        let n = buff1.get_i32(offset).unwrap();
        // should flush data to disk after displace
        buff1.set_i32(offset, n + 1).unwrap();
        buff1.set_modified(1, 0).unwrap();

        bm.unpin(buff1).unwrap();

        let buff2 = bm.pin(&BlockId::new("testfile", 2)).unwrap();
        bm.pin(&BlockId::new("testfile", 3)).unwrap();
        bm.pin(&BlockId::new("testfile", 4)).unwrap();

        let page = fm.read(&BlockId::new("testfile", 1)).unwrap();
        assert_eq!(1, page.get_i32(offset));

        bm.unpin(buff2).unwrap();
        let buff2 = bm.pin(&BlockId::new("testfile", 2)).unwrap();
        // should not flush data to disk
        buff2.set_i32(80, 9999).unwrap();
        buff2.set_modified(1, 0).unwrap();
        bm.unpin(buff2).unwrap();

        let page = fm.read(&BlockId::new("testfile", 1)).unwrap();
        assert_eq!(1, page.get_i32(offset));
    }
}
