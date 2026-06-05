use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use common::DbResult;

pub mod block;
pub(crate) mod holder;
pub mod mgr;
pub mod page;

pub(crate) fn open_file(filename: &Path) -> DbResult<File> {
    Ok(OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(filename)?)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::{block::BlockId, mgr::FileMgr, page::Page};

    #[test]
    fn file_mgr() {
        let file = tempdir().unwrap();
        let mgr = FileMgr::new(file.path(), 4096).unwrap();
        let block_id = BlockId::new("testfile", 2);
        let mut p1 = Page::new(mgr.block_size());
        let pos1 = 88;

        let str_value = "abcdefghjklm";

        p1.set_string(pos1, String::from(str_value));
        let size = Page::max_length(str_value);

        let i32_value = 345;

        let pos2 = pos1 + size;
        p1.set_i32(pos2, 345);
        mgr.write(block_id.clone(), p1).unwrap();

        let p2 = mgr.read(block_id).unwrap();

        assert_eq!(p2.get_i32(pos2), i32_value);
        assert_eq!(p2.get_string(pos1, str_value.len()), str_value);
    }
}
