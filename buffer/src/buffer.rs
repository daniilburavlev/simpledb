use std::sync::{Arc, Mutex, MutexGuard};

use common::{DbResult, error::DbError};
use file::{block::BlockId, mgr::FileMgr, page::Page};
use log::mgr::LogMgr;

struct BufferLock {
    fm: Arc<FileMgr>,
    lm: Arc<LogMgr>,
    contents: Page,
    block: Option<BlockId>,
    pins: i32,
    txnum: i32,
    lsn: i32,
}

impl BufferLock {
    fn new(fm: &Arc<FileMgr>, lm: &Arc<LogMgr>) -> DbResult<Self> {
        Ok(Self {
            fm: Arc::clone(fm),
            lm: Arc::clone(lm),
            contents: Page::new(fm.block_size()),
            block: None,
            pins: 0,
            txnum: -1,
            lsn: -1,
        })
    }

    fn block(&self) -> Option<BlockId> {
        self.block.clone()
    }

    fn is_pinned(&self) -> bool {
        self.pins > 0
    }

    fn modifying_tx(&self) -> i32 {
        self.txnum
    }

    fn set_modified(&mut self, txnum: i32, lsn: i32) {
        self.txnum = txnum;
        if lsn >= 0 {
            self.lsn = lsn
        }
    }

    fn assign_to_block(&mut self, block: &BlockId) -> DbResult<()> {
        self.flush()?;
        self.contents = self.fm.read(block)?;
        self.block = Some(block.clone());
        self.pins = 0;
        Ok(())
    }

    fn flush(&mut self) -> DbResult<()> {
        if self.txnum >= 0 {
            let Some(block) = &self.block else {
                return Err(DbError::EmtyBufferBlock);
            };
            self.lm.flush(self.lsn)?;
            self.fm.write(block, &self.contents)?;
            self.txnum = -1;
        }
        Ok(())
    }

    fn pin(&mut self) {
        self.pins += 1;
    }

    fn unpin(&mut self) {
        self.pins -= 1;
    }
}

pub struct BufferGuard<'a>(MutexGuard<'a, BufferLock>);

impl<'a> BufferGuard<'a> {
    pub fn block(&self) -> Option<BlockId> {
        self.0.block()
    }

    pub fn get_u8(&self, offset: usize) -> u8 {
        self.0.contents.get_u8(offset)
    }

    pub fn get_i32(&self, offset: usize) -> i32 {
        self.0.contents.get_i32(offset)
    }

    pub fn get_string(&self, offset: usize) -> String {
        self.0.contents.get_string(offset)
    }

    pub fn set_u8(&mut self, offset: usize, value: u8) {
        self.0.contents.set_u8(offset, value);
    }

    pub fn set_i32(&mut self, offset: usize, value: i32) {
        self.0.contents.set_i32(offset, value);
    }

    pub fn set_string(&mut self, offset: usize, value: &str) {
        self.0.contents.set_string(offset, value);
    }
    pub fn set_modified(&mut self, txnum: i32, lsn: i32) {
        self.0.set_modified(txnum, lsn);
    }
}

#[derive(Clone)]
pub struct Buffer {
    buffer: Arc<Mutex<BufferLock>>,
}

impl Buffer {
    pub fn new(fm: &Arc<FileMgr>, lm: &Arc<LogMgr>) -> DbResult<Self> {
        let buffer = Arc::new(Mutex::new(BufferLock::new(fm, lm)?));
        Ok(Self { buffer })
    }

    pub fn lock(&self) -> DbResult<BufferGuard<'_>> {
        let guard = self.buffer.lock().map_err(DbError::lock)?;
        Ok(BufferGuard(guard))
    }

    pub fn block(&self) -> DbResult<Option<BlockId>> {
        let lock = self.buffer.lock().map_err(DbError::lock)?;
        Ok(lock.block())
    }

    pub fn set_modified(&self, txnum: i32, lsn: i32) -> DbResult<()> {
        let mut lock = self.buffer.lock().map_err(DbError::lock)?;
        lock.set_modified(txnum, lsn);
        Ok(())
    }

    pub fn is_pinned(&self) -> DbResult<bool> {
        let lock = self.buffer.lock().map_err(DbError::lock)?;
        Ok(lock.is_pinned())
    }

    pub fn modifying_tx(&self) -> DbResult<i32> {
        let lock = self.buffer.lock().map_err(DbError::lock)?;
        Ok(lock.modifying_tx())
    }

    pub fn flush(&self) -> DbResult<()> {
        let mut lock = self.buffer.lock().map_err(DbError::lock)?;
        lock.flush()
    }

    pub fn assign_to_block(&self, block: &BlockId) -> DbResult<()> {
        let mut lock = self.buffer.lock().map_err(DbError::lock)?;
        lock.assign_to_block(block)
    }

    pub fn pin(&self) -> DbResult<()> {
        let mut lock = self.buffer.lock().map_err(DbError::lock)?;
        lock.pin();
        Ok(())
    }

    pub fn unpin(&self) -> DbResult<()> {
        let mut lock = self.buffer.lock().map_err(DbError::lock)?;
        lock.unpin();
        Ok(())
    }

    pub fn set_i32(&self, offset: usize, value: i32) -> DbResult<()> {
        let mut lock = self.buffer.lock().map_err(DbError::lock)?;
        lock.contents.set_i32(offset, value);
        Ok(())
    }

    pub fn get_i32(&self, offset: usize) -> DbResult<i32> {
        let lock = self.buffer.lock().map_err(DbError::lock)?;
        Ok(lock.contents.get_i32(offset))
    }

    pub fn set_string(&self, offset: usize, value: &str) -> DbResult<()> {
        let mut lock = self.buffer.lock().map_err(DbError::lock)?;
        lock.contents.set_string(offset, value);
        Ok(())
    }

    pub fn get_string(&self, offset: usize) -> DbResult<String> {
        let lock = self.buffer.lock().map_err(DbError::lock)?;
        Ok(lock.contents.get_string(offset))
    }

    pub fn set_bytes(&self, offset: usize, value: &[u8]) -> DbResult<()> {
        let mut lock = self.buffer.lock().map_err(DbError::lock)?;
        lock.contents.set_bytes(offset, value);
        Ok(())
    }

    pub fn get_bytes(&self, offset: usize) -> DbResult<Vec<u8>> {
        let lock = self.buffer.lock().map_err(DbError::lock)?;
        Ok(lock.contents.get_bytes(offset).to_vec())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn get_set() {
        let dir = tempdir().unwrap();
        let fm = Arc::new(FileMgr::new(dir.path(), 512).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "testlog".to_string()).unwrap());

        let buffer = Buffer::new(&fm, &lm).unwrap();

        buffer.set_string(20, "test").unwrap();
        let value = buffer.get_string(20).unwrap();
        assert_eq!("test", value);

        let bytes = vec![0u8; 15];
        buffer.set_bytes(100, &bytes).unwrap();
        let value = buffer.get_bytes(100).unwrap();
        assert_eq!(&bytes, &value);
    }

    #[test]
    #[should_panic]
    fn flush_empty() {
        let dir = tempdir().unwrap();
        let fm = Arc::new(FileMgr::new(dir.path(), 512).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "log".to_string()).unwrap());
        let mut buffer = BufferLock::new(&fm, &lm).unwrap();
        buffer.txnum = 1;
        buffer.flush().unwrap();
    }
}
