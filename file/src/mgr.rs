use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{Mutex, MutexGuard},
};

use common::{DbResult, error::DbError};

use crate::{block::BlockId, holder::FileHolder, page::Page};

pub struct FileMgr {
    block_size: usize,
    holder: Mutex<FileHolder>,
}

impl FileMgr {
    pub fn new(dir: &Path, block_size: usize) -> DbResult<Self> {
        fs::create_dir_all(dir)?;
        let holder = Mutex::new(FileHolder::new(dir));
        Ok(Self { block_size, holder })
    }

    pub fn read(&self, block_id: BlockId) -> DbResult<Page> {
        let mut open_files = self.holder.lock().unwrap();
        let mut fd = open_files.get(&block_id.filename)?;
        fd.seek(SeekFrom::Start(
            block_id.num as u64 * self.block_size as u64,
        ))?;
        let mut buffer = vec![0u8; self.block_size];
        fd.read_exact(&mut buffer).unwrap();
        Ok(Page::from(buffer.as_slice()))
    }

    pub fn write(&self, block_id: BlockId, page: Page) -> DbResult<()> {
        let mut open_files = self.holder.lock().map_err(DbError::lock)?;
        let mut fd = open_files.get(&block_id.filename)?;
        fd.seek(SeekFrom::Start(
            block_id.num as u64 * self.block_size as u64,
        ))?;
        fd.write_all(page.contents())?;
        Ok(())
    }

    pub fn append(&self, filename: &str) -> DbResult<BlockId> {
        let mut lock = self.holder.lock().map_err(DbError::lock)?;
        let block_num = size(&mut lock, filename)?;
        let block_id = BlockId::new(filename, block_num as usize);
        let mut fd = lock.get(filename)?;
        let buffer = vec![0u8; self.block_size];
        fd.seek(SeekFrom::End(0))?;
        fd.write_all(&buffer)?;
        Ok(block_id)
    }

    pub fn size(&self, filename: &str) -> DbResult<u64> {
        let mut lock = self.holder.lock().map_err(DbError::lock)?;
        size(&mut lock, filename)
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

fn size(lock: &mut MutexGuard<'_, FileHolder>, filename: &str) -> DbResult<u64> {
    let mut fd = lock.get(filename)?;
    let size = fd.seek(SeekFrom::End(0))?;
    Ok(size)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn append_size() {
        let dir = tempdir().unwrap();
        let size = 256;
        let mgr = FileMgr::new(dir.path(), size).unwrap();
        mgr.append("test").unwrap();
        let calculated = mgr.size("test").unwrap();
        assert_eq!(calculated, size as u64);
    }
}
