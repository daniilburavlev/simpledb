use buffer::mgr::BufferMgr;
use common::{DbResult, error::DbError};
use file::{block::BlockId, mgr::FileMgr};
use log::mgr::LogMgr;
use std::sync::{
    Arc,
    atomic::{AtomicI32, Ordering},
};

use crate::{
    buffer_list::BufferList, concurrency::mgr::ConcurrencyMgr, lock_table::LockTable,
    recovery::mgr::RecoveryMgr,
};

static TX_NUM: AtomicI32 = AtomicI32::new(0);
const END_OF_FILE: i32 = -1;

fn next_tx_num() -> i32 {
    TX_NUM.fetch_add(1, Ordering::SeqCst)
}

pub struct Transaction {
    concurrency_mgr: ConcurrencyMgr,
    fm: Arc<FileMgr>,
    rm: RecoveryMgr,
    bm: Arc<BufferMgr>,
    txnum: i32,
    buffers: BufferList,
}

impl Transaction {
    pub fn new(
        fm: &Arc<FileMgr>,
        lm: &Arc<LogMgr>,
        bm: &Arc<BufferMgr>,
        lock_table: &Arc<LockTable>,
    ) -> DbResult<Self> {
        let txnum = next_tx_num();
        let concurrency_mgr = ConcurrencyMgr::new(lock_table);
        let rm = RecoveryMgr::new(txnum, lm, bm)?;
        Ok(Self {
            txnum,
            fm: Arc::clone(fm),
            concurrency_mgr,
            bm: Arc::clone(bm),
            rm,
            buffers: BufferList::new(bm),
        })
    }

    pub fn commit(&self) -> DbResult<()> {
        self.rm.commit()?;
        self.concurrency_mgr.release()?;
        self.buffers.unpin_all()?;
        tracing::debug!("transaction '{}' commited", self.txnum);
        Ok(())
    }

    pub fn rollback(&self) -> DbResult<()> {
        self.rm.rollback(self)?;
        self.concurrency_mgr.release()?;
        self.buffers.unpin_all()?;
        tracing::debug!("transaction '{}' rolled back", self.txnum);
        Ok(())
    }

    pub fn recover(&self) -> DbResult<()> {
        self.bm.flush_all(self.txnum)?;
        self.rm.recover(self)?;
        Ok(())
    }

    pub fn pin(&self, block: &BlockId) -> DbResult<()> {
        self.buffers.pin(block)?;
        Ok(())
    }

    pub fn unpin(&self, block: &BlockId) -> DbResult<()> {
        self.buffers.unpin(block)?;
        Ok(())
    }

    pub fn set_u8(
        &self,
        block: &BlockId,
        offset: usize,
        value: u8,
        ok_to_log: bool,
    ) -> DbResult<()> {
        self.concurrency_mgr.x_lock(block)?;
        let Some(buffer) = self.buffers.get_buffer(block)? else {
            return Err(DbError::UnexistedBuffer);
        };
        let mut guard = buffer.lock()?;
        let lsn = if ok_to_log {
            self.rm.set_u8(&guard, offset, value)?
        } else {
            -1
        };
        guard.set_u8(offset, value);
        guard.set_modified(self.txnum, lsn);
        Ok(())
    }

    pub fn set_i32(
        &self,
        block: &BlockId,
        offset: usize,
        value: i32,
        ok_to_log: bool,
    ) -> DbResult<()> {
        self.concurrency_mgr.x_lock(block)?;
        let Some(buffer) = self.buffers.get_buffer(block)? else {
            return Err(DbError::UnexistedBuffer);
        };
        let mut guard = buffer.lock()?;
        let lsn = if ok_to_log {
            self.rm.set_i32(&guard, offset, value)?
        } else {
            -1
        };
        guard.set_i32(offset, value);
        guard.set_modified(self.txnum, lsn);
        Ok(())
    }

    pub fn get_u8(&self, block: &BlockId, offset: usize) -> DbResult<u8> {
        self.concurrency_mgr.s_lock(block)?;
        let Some(buffer) = self.buffers.get_buffer(block)? else {
            return Err(DbError::BufferAbort);
        };
        buffer.get_u8(offset)
    }

    pub fn get_i32(&self, block: &BlockId, offset: usize) -> DbResult<i32> {
        self.concurrency_mgr.s_lock(block)?;
        let Some(buffer) = self.buffers.get_buffer(block)? else {
            return Err(DbError::BufferAbort);
        };
        buffer.get_i32(offset)
    }

    pub fn set_string(
        &self,
        block: &BlockId,
        offset: usize,
        value: &str,
        ok_to_log: bool,
    ) -> DbResult<()> {
        self.concurrency_mgr.x_lock(block)?;
        let Some(buffer) = self.buffers.get_buffer(block)? else {
            return Err(DbError::UnexistedBuffer);
        };
        let mut guard = buffer.lock()?;
        let lsn = if ok_to_log {
            self.rm.set_string(&guard, offset, value)?
        } else {
            -1
        };
        guard.set_string(offset, value);
        guard.set_modified(self.txnum, lsn);
        Ok(())
    }

    pub fn get_string(&self, block: &BlockId, offset: usize) -> DbResult<String> {
        self.concurrency_mgr.s_lock(block)?;
        let Some(buffer) = self.buffers.get_buffer(block)? else {
            return Err(DbError::BufferAbort);
        };
        buffer.get_string(offset)
    }

    pub fn available_buffs(&self) -> DbResult<u32> {
        self.bm.available()
    }

    pub fn size(&self, filename: &str) -> DbResult<u64> {
        let dummy = BlockId::new(filename, END_OF_FILE);
        self.concurrency_mgr.s_lock(&dummy)?;
        let length = self.fm.length(filename)?;
        tracing::debug!("file size: {}", length);
        Ok(length)
    }

    pub fn append(&self, filename: &str) -> DbResult<BlockId> {
        let dummy = BlockId::new(filename, -1);
        self.concurrency_mgr.x_lock(&dummy)?;
        self.fm.append(filename)
    }

    pub fn block_size(&self) -> i32 {
        self.fm.block_size()
    }

    pub fn txnum(&self) -> i32 {
        self.txnum
    }
}

#[cfg(test)]
mod tests {

    use file::mgr::FileMgr;
    use tempfile::tempdir;

    use crate::lock_table::LockTable;

    fn setup(dir: &std::path::Path) -> (Arc<FileMgr>, Arc<LogMgr>, Arc<BufferMgr>, Arc<LockTable>) {
        let fm = Arc::new(FileMgr::new(dir, 512).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "testlog".to_string()).unwrap());
        let bm = Arc::new(BufferMgr::new(&fm, &lm, 8).unwrap());
        let lock_table = Arc::new(LockTable::default());
        (fm, lm, bm, lock_table)
    }

    use super::*;

    #[test]
    fn recover_undoes_uncommitted() {
        let dir = tempdir().unwrap();
        let (fm, lm, bm, lock_table) = setup(dir.path());

        // allocate a real block on disk
        let block = fm.append("testfile").unwrap();

        // tx1: write initial value 100, commit (clean baseline)
        let tx1 = Transaction::new(&fm, &lm, &bm, &lock_table).unwrap();
        tx1.pin(&block).unwrap();
        tx1.set_i32(&block, 0, 100, false).unwrap();
        tx1.commit().unwrap();

        // tx2: overwrite with 999 (logged), then flush to disk — simulates a crash
        let tx2 = Transaction::new(&fm, &lm, &bm, &lock_table).unwrap();
        tx2.pin(&block).unwrap();
        tx2.set_i32(&block, 0, 999, true).unwrap();
        bm.flush_all(tx2.txnum()).unwrap();
        // no commit — crash; drop tx2 without releasing locks
        drop(tx2);

        // after a crash the lock table is reset (server restart)
        let lock_table = Arc::new(LockTable::default());

        // tx3: recovery should undo tx2's write and restore 100
        let tx3 = Transaction::new(&fm, &lm, &bm, &lock_table).unwrap();
        tx3.recover().unwrap();
        tx3.commit().unwrap();

        // tx4: verify the value was restored
        let tx4 = Transaction::new(&fm, &lm, &bm, &lock_table).unwrap();
        tx4.pin(&block).unwrap();
        let val = tx4.get_i32(&block, 0).unwrap();
        tx4.commit().unwrap();

        assert_eq!(val, 100);
    }

    #[test]
    fn recover_preserves_committed() {
        let dir = tempdir().unwrap();
        let (fm, lm, bm, lock_table) = setup(dir.path());

        let block = fm.append("testfile").unwrap();

        // tx1: write 42, commit
        let tx1 = Transaction::new(&fm, &lm, &bm, &lock_table).unwrap();
        tx1.pin(&block).unwrap();
        tx1.set_i32(&block, 0, 42, true).unwrap();
        tx1.commit().unwrap();

        // tx2: recovery — tx1 was committed, so nothing should be undone
        let tx2 = Transaction::new(&fm, &lm, &bm, &lock_table).unwrap();
        tx2.pin(&block).unwrap();
        tx2.recover().unwrap();

        let tx3 = Transaction::new(&fm, &lm, &bm, &lock_table).unwrap();
        tx3.pin(&block).unwrap();
        let val = tx3.get_i32(&block, 0).unwrap();
        tx3.commit().unwrap();

        assert_eq!(val, 42);
    }
}
