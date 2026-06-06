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

    pub fn read(&self, block_id: &BlockId) -> DbResult<Page> {
        let mut open_files = self.holder.lock().unwrap();
        let length = length(&mut open_files, &block_id.filename)?;
        let mut fd = open_files.get(&block_id.filename)?;
        fd.seek(SeekFrom::Start(
            block_id.num as u64 * self.block_size as u64,
        ))?;
        let mut buffer = vec![0u8; self.block_size];
        let required = (self.block_size * block_id.num) as u64;
        if length > required {
            fd.read_exact(&mut buffer)?;
        }
        Ok(Page::from(buffer.as_slice()))
    }

    pub fn write(&self, block_id: &BlockId, page: &Page) -> DbResult<()> {
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
        let block_num = size(&mut lock, filename, self.block_size as u64)?;
        let block_id = BlockId::new(filename, block_num as usize);
        let mut fd = lock.get(filename)?;
        let buffer = vec![0u8; self.block_size];
        fd.seek(SeekFrom::End(0))?;
        fd.write_all(&buffer)?;
        Ok(block_id)
    }

    pub fn length(&self, filename: &str) -> DbResult<u64> {
        let mut lock = self.holder.lock().map_err(DbError::lock)?;
        length(&mut lock, filename)
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

fn size(lock: &mut MutexGuard<'_, FileHolder>, filename: &str, block_size: u64) -> DbResult<u64> {
    let length = length(lock, filename)?;
    Ok(length / block_size)
}

fn length(lock: &mut MutexGuard<'_, FileHolder>, filename: &str) -> DbResult<u64> {
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
        let calculated = mgr.length("test").unwrap();
        assert_eq!(calculated, size as u64);
    }

    #[test]
    fn file_mgr() {
        let file = tempdir().unwrap();
        let mgr = FileMgr::new(file.path(), 4096).unwrap();
        let block_id = BlockId::new("testfile", 2);
        let mut p1 = Page::new(mgr.block_size());
        let pos1 = 88;

        let str_value = "abcdefghjklm";

        p1.set_string(pos1, String::from(str_value));
        let size = Page::str_space(str_value);

        let i32_value = 345;

        let pos2 = pos1 + size;
        p1.set_i32(pos2, 345);
        mgr.write(&block_id, &p1).unwrap();

        let p2 = mgr.read(&block_id).unwrap();

        assert_eq!(p2.get_i32(pos2), i32_value);
        assert_eq!(p2.get_string(pos1), str_value);
    }
}
