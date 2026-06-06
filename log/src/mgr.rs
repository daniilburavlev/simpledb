use std::sync::{Arc, Mutex};

use common::{DbResult, error::DbError};
use file::{block::BlockId, mgr::FileMgr, page::Page};
use file::page::U16_SIZE;

struct Log {
    fm: Arc<FileMgr>,
    logfile: String,
    logpage: Page,
    current_block: BlockId,
    latest_lsn: u64,
    last_saved_lsn: u64,
}

impl Log {
    fn new(fm: &Arc<FileMgr>, logfile: String) -> DbResult<Self> {
        let logsize = fm.length(&logfile)?;
        let (current_block, logpage) = if logsize == 0 {
            let mut page = Page::new(fm.block_size());
            let block = fm.append(&logfile)?;
            page.set_u16(0, fm.block_size() as u16);
            fm.write(&block, &page)?;
            (block, page)
        } else {
            let block = BlockId::new(&logfile, logsize as usize - 1);
            let page = fm.read(&block)?;
            (block, page)
        };
        Ok(Self {
            fm: Arc::clone(fm),
            logfile,
            logpage,
            current_block,
            latest_lsn: 0,
            last_saved_lsn: 0,
        })
    }

    fn append(&mut self, logrec: &[u8]) -> DbResult<u64> {
        let mut boundary = self.logpage.get_u16(0) as usize;
        let bytesneeded = Page::bytes_space(logrec.len());
        if bytesneeded + U16_SIZE > boundary {
            self._flush()?;
            self.current_block = self.append_block()?;
            boundary = self.logpage.get_u16(0) as usize;
        }
        let recpos = boundary - bytesneeded;
        self.logpage.set_bytes(recpos, logrec);
        self.logpage.set_u16(0, recpos as u16);
        self.latest_lsn += 1;
        Ok(self.latest_lsn)
    }

    fn append_block(&mut self) -> DbResult<BlockId> {
        let block = self.fm.append(&self.logfile)?;
        self.logpage.set_u16(0, self.fm.block_size() as u16);
        self.fm.write(&block, &self.logpage)?;
        Ok(block)
    }

    fn flush(&mut self, lsn: u64) -> DbResult<()> {
        if lsn > self.last_saved_lsn {
            self._flush()?;
        }
        Ok(())
    }

    fn _flush(&mut self) -> DbResult<()> {
        self.fm.write(&self.current_block, &self.logpage)?;
        self.last_saved_lsn = self.latest_lsn;
        Ok(())
    }

    fn iter(&mut self) -> DbResult<LogIterator> {
        self._flush()?;
        LogIterator::new(&self.fm, self.current_block.clone())
    }
}

struct LogIterator {
    fm: Arc<FileMgr>,
    block: BlockId,
    page: Page,
    current_pos: usize,
    boundary: usize,
}

impl LogIterator {
    fn new(fm: &Arc<FileMgr>, block: BlockId) -> DbResult<Self> {
        let page = fm.read(&block)?;
        let boundary = page.get_u16(0) as usize;
        Ok(Self {
            fm: Arc::clone(fm),
            block,
            page,
            boundary,
            current_pos: boundary,
        })
    }
}

impl Iterator for LogIterator {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_pos >= self.fm.block_size() && self.block.num == 0 {
            return None;
        }
        if self.current_pos == self.fm.block_size() {
            self.block = BlockId::new(&self.block.filename, self.block.num - 1);
            self.page = self.fm.read(&self.block).ok()?;
            self.boundary = self.page.get_u16(0) as usize;
            self.current_pos = self.boundary;
        }
        let rec = self.page.get_bytes(self.current_pos);
        self.current_pos += Page::bytes_space(rec.len());
        Some(rec.to_vec())
    }
}

pub struct LogMgr {
    log: Mutex<Log>,
}

impl LogMgr {
    pub fn new(fm: &Arc<FileMgr>, logfile: String) -> DbResult<Self> {
        let log = Log::new(fm, logfile)?;
        Ok(Self {
            log: Mutex::new(log),
        })
    }

    pub fn append(&self, logrec: &[u8]) -> DbResult<u64> {
        let mut lock = self.log.lock().map_err(DbError::lock)?;
        lock.append(logrec)
    }

    pub fn flush(&self, lsn: u64) -> DbResult<()> {
        let mut lock = self.log.lock().map_err(DbError::lock)?;
        lock.flush(lsn)
    }

    fn iter(&self) -> DbResult<LogIterator> {
        let mut lock = self.log.lock().map_err(DbError::lock)?;
        lock.iter()
    }
}

#[cfg(test)]
mod tests {

    use tempfile::tempdir;
    use super::*;

    #[test]
    fn print_log() {
        let dir = tempdir().unwrap();
        let file_mgr = Arc::new(FileMgr::new(dir.path(), 64).unwrap());

        let log_mgr = LogMgr::new(&file_mgr, "log".to_string()).unwrap();
        create_records(&log_mgr, 1, 35);
        print_log_records(&log_mgr);
        create_records(&log_mgr, 36, 70);
        log_mgr.flush(65).unwrap();
        print_log_records(&log_mgr);
    }

    fn print_log_records(lm: &LogMgr) {
        for bytes in lm.iter().unwrap() {
            let page = Page::from(bytes.as_slice());
            let record = page.get_string(0);
            println!("record: {} - {}", record, page.get_u16(Page::str_space(&record)))
        }
    }

    fn create_records(lm: &LogMgr, start: usize, end: usize) {
        for i in start..end {
            let (rec, size) = create_log_record(i);
            let lsn = lm.append(rec.as_slice()).unwrap();
            print!("{} ", lsn);
        }
        println!();
    }

    fn create_log_record(i: usize) -> (Vec<u8>, usize) {
        let s = format!("record{}", i);
        let npos = Page::str_space(&s);
        let mut p = Page::new(npos + U16_SIZE);
        p.set_string(0, s);
        p.set_u16(npos, i as u16 + 100);
        (p.contents().to_vec(), npos)
    }
}
