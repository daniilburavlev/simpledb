use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{Mutex, MutexGuard},
};

use common::{DbResult, error::DbError};

use crate::{block::BlockId, holder::FileHolder, page::Page};

pub struct FileMgr {
    block_size: i32,
    holder: Mutex<FileHolder>,
}

impl FileMgr {
    pub fn new(dir: &Path, block_size: i32) -> DbResult<Self> {
        fs::create_dir_all(dir)?;
        let holder = Mutex::new(FileHolder::new(dir));
        Ok(Self { block_size, holder })
    }

    pub fn is_new(&self) -> DbResult<bool> {
        let holder = self.holder.lock().map_err(DbError::lock)?;
        let mut entries = fs::read_dir(&holder.dir)?;
        Ok(entries.next().is_some())
    }

    pub fn read(&self, block_id: &BlockId) -> DbResult<Page> {
        let mut open_files = self.holder.lock().unwrap();
        let num_blocks = length(&mut open_files, &block_id.filename, self.block_size)?;
        let mut fd = open_files.get(&block_id.filename)?;
        fd.seek(SeekFrom::Start(
            block_id.num as u64 * self.block_size as u64,
        ))?;
        let mut buffer = vec![0u8; self.block_size as usize];
        if num_blocks > block_id.num as u64 {
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
        let block_num = length(&mut lock, filename, self.block_size)?;
        let block_id = BlockId::new(filename, block_num as i32);
        let mut fd = lock.get(filename)?;
        let buffer = vec![0u8; self.block_size as usize];
        fd.seek(SeekFrom::End(0))?;
        fd.write_all(&buffer)?;
        Ok(block_id)
    }

    pub fn length(&self, filename: &str) -> DbResult<u64> {
        let mut lock = self.holder.lock().map_err(DbError::lock)?;
        length(&mut lock, filename, self.block_size)
    }

    pub fn block_size(&self) -> i32 {
        self.block_size
    }
}

fn length(lock: &mut MutexGuard<'_, FileHolder>, filename: &str, block_size: i32) -> DbResult<u64> {
    let mut fd = lock.get(filename)?;
    let size = fd.seek(SeekFrom::End(0))?;
    Ok(size / block_size as u64)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn append_size() {
        let dir = tempdir().unwrap();
        let block_size = 256;
        let mgr = FileMgr::new(dir.path(), block_size).unwrap();
        mgr.append("test").unwrap();
        // length() now reports the file size in blocks, not bytes
        let calculated = mgr.length("test").unwrap();
        assert_eq!(calculated, 1);
    }

    #[test]
    fn file_mgr() {
        let file = tempdir().unwrap();
        let mgr = FileMgr::new(file.path(), 4096).unwrap();
        let block_id = BlockId::new("testfile", 2);
        let mut p1 = Page::new(mgr.block_size());
        let pos1 = 88;

        let str_value = "abcdefghjklm";

        p1.set_string(pos1, str_value);
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
