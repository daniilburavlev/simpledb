use buffer::mgr::BufferMgr;
use common::{DbResult, error::DbError};
use file::{block::BlockId, mgr::FileMgr};
use log::mgr::LogMgr;
use std::sync::Arc;

use crate::{
    buffer_list::BufferList, concurrency::mgr::ConcurrencyMgr, lock_table::LockTable,
    recovery::mgr::RecoveryMgr, txnum_generator::TxNumGenerator,
};

const END_OF_FILE: i32 = -1;

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
        txnum_generator: &TxNumGenerator,
        fm: &Arc<FileMgr>,
        lm: &Arc<LogMgr>,
        bm: &Arc<BufferMgr>,
        lock_table: &Arc<LockTable>,
    ) -> DbResult<Self> {
        let concurrency_mgr = ConcurrencyMgr::new(lock_table);
        let txnum = txnum_generator.next();
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
        let lsn = if ok_to_log {
            self.rm.set_i32(&buffer, offset, value)?
        } else {
            -1
        };
        buffer.set_i32(offset, value)?;
        buffer.set_modified(self.txnum, lsn)
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
        value: String,
        ok_to_log: bool,
    ) -> DbResult<()> {
        self.concurrency_mgr.x_lock(block)?;
        let Some(buffer) = self.buffers.get_buffer(block)? else {
            return Err(DbError::UnexistedBuffer);
        };
        let lsn = if ok_to_log {
            self.rm.set_string(&buffer, offset, value.clone())?
        } else {
            -1
        };
        buffer.set_string(offset, value)?;
        buffer.set_modified(self.txnum, lsn)
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
        self.fm.length(filename)
    }

    pub fn append(&self, filename: &str) -> DbResult<BlockId> {
        let dummy = BlockId::new(filename, -1);
        self.concurrency_mgr.x_lock(&dummy)?;
        self.fm.append(filename)
    }

    pub fn block_size(&self) -> usize {
        self.fm.block_size()
    }
}
