use common::{DbResult, error::DbError};
use file::{
    block::BlockId,
    page::{I32_SIZE, Page},
};
use log::mgr::LogMgr;

use crate::transaction::Transaction;

const CHECKPOINT_TXNUM: i32 = -1;
const CHECKPOINT: i32 = 0;
const START: i32 = 1;
const COMMIT: i32 = 2;
const ROLLBACK: i32 = 3;
const SETSTRING: i32 = 4;
const SETI32: i32 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogRecord {
    Checkpoint,
    Start(i32),
    Commit(i32),
    Rollback(i32),
    SetI32 {
        txnum: i32,
        offset: usize,
        value: i32,
        block: BlockId,
    },
    SetString {
        txnum: i32,
        offset: usize,
        value: String,
        block: BlockId,
    },
}

impl LogRecord {
    pub fn new(bytes: &[u8]) -> DbResult<Self> {
        let page = Page::from(bytes);
        let log_type = page.get_i32(0);
        match log_type {
            CHECKPOINT => Ok(Self::checkpoint()),
            START => Ok(Self::start(&page)),
            COMMIT => Ok(Self::commit(&page)),
            ROLLBACK => Ok(Self::rollback(&page)),
            SETI32 => Ok(Self::set_i32(&page)),
            SETSTRING => Ok(Self::set_string(&page)),
            _ => Err(DbError::Decoding),
        }
    }

    fn checkpoint() -> Self {
        Self::Checkpoint
    }

    fn start(page: &Page) -> Self {
        let txnum_pos = I32_SIZE;
        let txnum = page.get_i32(txnum_pos);
        Self::Start(txnum)
    }

    fn commit(page: &Page) -> Self {
        let txnum_pos = I32_SIZE;
        let txnum = page.get_i32(txnum_pos);
        Self::Commit(txnum)
    }

    fn rollback(page: &Page) -> Self {
        let txnum_pos = I32_SIZE;
        let txnum = page.get_i32(txnum_pos);
        Self::Rollback(txnum)
    }

    fn set_i32(page: &Page) -> Self {
        let tpos = I32_SIZE;
        let txnum = page.get_i32(tpos);
        let fpos = tpos + I32_SIZE;
        let filename = page.get_string(fpos);
        let bpos = fpos + Page::str_space(&filename);
        let block_num = page.get_i32(bpos);
        let block = BlockId::new(&filename, block_num);
        let opos = bpos + I32_SIZE;
        let offset = page.get_i32(opos) as usize;
        let value_pos = opos + I32_SIZE;
        let value = page.get_i32(value_pos);
        Self::SetI32 {
            txnum,
            offset,
            value,
            block,
        }
    }

    pub fn set_string(page: &Page) -> Self {
        let tpos = I32_SIZE;
        let txnum = page.get_i32(tpos);
        let fpos = tpos + I32_SIZE;
        let filename = page.get_string(fpos);
        let bpos = fpos + Page::str_space(&filename);
        let block_num = page.get_i32(bpos);
        let block = BlockId::new(&filename, block_num);
        let opos = bpos + I32_SIZE;
        let offset = page.get_i32(opos) as usize;
        let value_pos = opos + I32_SIZE;
        let value = page.get_string(value_pos);
        Self::SetString {
            txnum,
            offset,
            value,
            block,
        }
    }

    pub fn op(&self) -> i32 {
        match self {
            Self::Checkpoint => CHECKPOINT,
            Self::Start(_) => START,
            Self::Commit(_) => COMMIT,
            Self::Rollback(_) => ROLLBACK,
            Self::SetI32 { .. } => SETI32,
            Self::SetString { .. } => SETSTRING,
        }
    }

    pub fn is_start(&self) -> bool {
        self.op() == START
    }

    pub fn is_checkpoint(&self) -> bool {
        self.op() == CHECKPOINT
    }

    pub fn is_commit(&self) -> bool {
        self.op() == COMMIT
    }

    pub fn is_rollback(&self) -> bool {
        self.op() == ROLLBACK
    }

    pub fn tx_number(&self) -> i32 {
        match self {
            Self::Checkpoint => CHECKPOINT_TXNUM,
            Self::Start(txnum) => *txnum,
            Self::Commit(txnum) => *txnum,
            Self::SetI32 { txnum, .. } => *txnum,
            Self::SetString { txnum, .. } => *txnum,
            _ => -1,
        }
    }

    pub fn undo(&self, tx: &Transaction) -> DbResult<()> {
        match self {
            Self::SetI32 {
                txnum: _,
                offset,
                value,
                block,
            } => {
                tx.pin(block)?;
                tx.set_i32(block, *offset, *value, false)?;
                tx.unpin(block)?;
            }
            Self::SetString {
                txnum: _,
                offset,
                value,
                block,
            } => {
                tx.pin(block)?;
                tx.set_string(block, *offset, value, false)?;
                tx.unpin(block)?;
            }
            _ => {}
        }
        Ok(())
    }
}

pub fn write_commit_to_log(lm: &LogMgr, txnum: i32) -> DbResult<i32> {
    write_op_txnum(lm, COMMIT, txnum)
}

pub fn write_rollback_to_log(lm: &LogMgr, txnum: i32) -> DbResult<i32> {
    write_op_txnum(lm, ROLLBACK, txnum)
}

pub fn write_start_to_log(lm: &LogMgr, txnum: i32) -> DbResult<i32> {
    write_op_txnum(lm, START, txnum)
}

pub fn write_checkpoint(lm: &LogMgr) -> DbResult<i32> {
    let mut page = Page::new(I32_SIZE.try_into().unwrap());
    page.set_i32(0, CHECKPOINT);
    lm.append(page.contents())
}

fn write_op_txnum(lm: &LogMgr, op: i32, txnum: i32) -> DbResult<i32> {
    let mut page = Page::new((I32_SIZE + I32_SIZE).try_into().unwrap());
    page.set_i32(0, op);
    page.set_i32(I32_SIZE, txnum);
    lm.append(page.contents())
}

pub fn write_u8_to_log(
    lm: &LogMgr,
    txnum: i32,
    block: &BlockId,
    offset: usize,
    value: u8,
) -> DbResult<i32> {
    let txnum_pos = I32_SIZE;
    let filename_pos = txnum_pos + I32_SIZE;
    let block_pos = filename_pos + Page::str_space(&block.filename);
    let offset_pos = block_pos + I32_SIZE;
    let value_pos = offset_pos + I32_SIZE;
    let rec_len = value_pos + I32_SIZE;
    let mut page = Page::new(rec_len.try_into()?);
    page.set_i32(0, SETI32);
    page.set_i32(txnum_pos, txnum);
    page.set_string(filename_pos, &block.filename);
    page.set_i32(block_pos, block.num);
    page.set_i32(offset_pos, offset.try_into()?);
    page.set_i32(value_pos, value.into());
    lm.append(page.contents())
}

pub fn write_i32_to_log(
    lm: &LogMgr,
    txnum: i32,
    block: &BlockId,
    offset: usize,
    value: i32,
) -> DbResult<i32> {
    let txnum_pos = I32_SIZE;
    let filename_pos = txnum_pos + I32_SIZE;
    let block_pos = filename_pos + Page::str_space(&block.filename);
    let offset_pos = block_pos + I32_SIZE;
    let value_pos = offset_pos + I32_SIZE;
    let rec_len = value_pos + I32_SIZE;
    let mut page = Page::new(rec_len.try_into()?);
    page.set_i32(0, SETI32);
    page.set_i32(txnum_pos, txnum);
    page.set_string(filename_pos, &block.filename);
    page.set_i32(block_pos, block.num);
    page.set_i32(offset_pos, offset.try_into()?);
    page.set_i32(value_pos, value);
    lm.append(page.contents())
}

pub fn write_string_to_log(
    lm: &LogMgr,
    txnum: i32,
    block: &BlockId,
    offset: usize,
    value: String,
) -> DbResult<i32> {
    let txnum_pos = I32_SIZE;
    let filename_pos = txnum_pos + I32_SIZE;
    let block_pos = filename_pos + Page::str_space(&block.filename);
    let offset_pos = block_pos + I32_SIZE;
    let value_pos = offset_pos + I32_SIZE;
    let rec_len = value_pos + Page::str_space(&value);
    let mut page = Page::new(rec_len.try_into()?);
    page.set_i32(0, SETSTRING);
    page.set_i32(txnum_pos, txnum);
    page.set_string(filename_pos, &block.filename);
    page.set_i32(block_pos, block.num);
    page.set_i32(offset_pos, offset.try_into()?);
    page.set_string(value_pos, &value);
    lm.append(page.contents())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use file::mgr::FileMgr;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn write_read() {
        let dir = tempdir().unwrap();
        let fm = FileMgr::new(dir.path(), 512).unwrap();
        let fm = Arc::new(fm);
        let lm = LogMgr::new(&fm, "testlog".to_string()).unwrap();
        let lm = Arc::new(lm);

        let str_block = BlockId::new("test", 100);
        write_string_to_log(&lm, 4, &str_block, 80, "value".to_string()).unwrap();
        let i32_block = BlockId::new("test", 0);
        write_i32_to_log(&lm, 4, &i32_block, 0, 1337).unwrap();
        write_rollback_to_log(&lm, 3).unwrap();
        write_commit_to_log(&lm, 2).unwrap();
        write_start_to_log(&lm, 1).unwrap();
        write_checkpoint(&lm).unwrap();

        let mut iter = lm.iter().unwrap();
        let bytes = iter.next().unwrap();
        let checkpoint = LogRecord::new(&bytes).unwrap();
        assert_eq!(checkpoint, LogRecord::Checkpoint);

        let bytes = iter.next().unwrap();
        let start = LogRecord::new(&bytes).unwrap();
        assert_eq!(start, LogRecord::Start(1));

        let bytes = iter.next().unwrap();
        let commit = LogRecord::new(&bytes).unwrap();
        assert_eq!(commit, LogRecord::Commit(2));

        let bytes = iter.next().unwrap();
        let rollback = LogRecord::new(&bytes).unwrap();
        assert_eq!(rollback, LogRecord::Rollback(3));

        let bytes = iter.next().unwrap();
        let value_i32 = LogRecord::new(&bytes).unwrap();
        assert_eq!(
            value_i32,
            LogRecord::SetI32 {
                txnum: 4,
                offset: 0,
                value: 1337,
                block: i32_block
            }
        );

        let bytes = iter.next().unwrap();
        let value_i32 = LogRecord::new(&bytes).unwrap();
        assert_eq!(
            value_i32,
            LogRecord::SetString {
                txnum: 4,
                offset: 80,
                value: "value".to_string(),
                block: str_block
            }
        );
    }
}
