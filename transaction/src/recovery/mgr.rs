use std::{collections::HashSet, sync::Arc};

use buffer::{buffer::BufferGuard, mgr::BufferMgr};
use common::{DbResult, error::DbError};
use log::mgr::LogMgr;

use crate::{
    log::{
        LogRecord, write_checkpoint, write_commit_to_log, write_i32_to_log, write_rollback_to_log,
        write_string_to_log,
    },
    transaction::Transaction,
};

pub struct RecoveryMgr {
    lm: Arc<LogMgr>,
    bm: Arc<BufferMgr>,
    txnum: i32,
}

impl RecoveryMgr {
    pub fn new(txnum: i32, lm: &Arc<LogMgr>, bm: &Arc<BufferMgr>) -> DbResult<Self> {
        Ok(Self {
            lm: Arc::clone(lm),
            bm: Arc::clone(bm),
            txnum,
        })
    }

    pub fn commit(&self) -> DbResult<()> {
        self.bm.flush_all(self.txnum)?;
        let lsn = write_commit_to_log(&self.lm, self.txnum)?;
        self.lm.flush(lsn)
    }

    pub fn rollback(&self, tx: &Transaction) -> DbResult<()> {
        self.do_rollback(tx)?;
        self.bm.flush_all(self.txnum)?;
        let lsn = write_rollback_to_log(&self.lm, self.txnum)?;
        self.lm.flush(lsn)
    }

    pub fn recover(&self, tx: &Transaction) -> DbResult<()> {
        self.do_recover(tx)?;
        self.bm.flush_all(self.txnum)?;
        let lsn = write_checkpoint(&self.lm)?;
        self.lm.flush(lsn)
    }

    pub fn set_i32(&self, buffer: &BufferGuard<'_>, offset: usize, _value: i32) -> DbResult<i32> {
        let old_val = buffer.get_i32(offset);
        let Some(block) = buffer.block() else {
            return Err(DbError::EmtyBufferBlock);
        };
        write_i32_to_log(&self.lm, self.txnum, &block, offset, old_val)
    }

    pub fn set_string(
        &self,
        buffer: &BufferGuard<'_>,
        offset: usize,
        _value: &str,
    ) -> DbResult<i32> {
        let old_val = buffer.get_string(offset);
        let Some(block) = buffer.block() else {
            return Err(DbError::EmtyBufferBlock);
        };
        write_string_to_log(&self.lm, self.txnum, &block, offset, old_val)
    }

    fn do_rollback(&self, tx: &Transaction) -> DbResult<()> {
        for bytes in self.lm.iter()? {
            let rec = LogRecord::new(&bytes)?;
            if rec.tx_number() == self.txnum {
                if rec.is_start() {
                    return Ok(());
                }
                rec.undo(tx)?;
            }
        }
        Ok(())
    }

    fn do_recover(&self, tx: &Transaction) -> DbResult<()> {
        let mut finished_txs = HashSet::new();
        for bytes in self.lm.iter()? {
            let rec = LogRecord::new(&bytes)?;
            if rec.is_checkpoint() {
                return Ok(());
            } else if rec.is_commit() || rec.is_rollback() {
                finished_txs.insert(rec.tx_number());
            } else if !finished_txs.contains(&rec.tx_number()) {
                rec.undo(tx)?;
            }
        }
        Ok(())
    }
}
