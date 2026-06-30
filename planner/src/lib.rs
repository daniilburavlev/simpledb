pub mod element;
pub mod field_info;
pub mod index;
pub mod layout;
pub mod mgr;
pub mod plan;
pub mod predicate;
pub mod record_page;
pub mod rid;
pub mod scan;
pub mod schema;
pub mod value;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use buffer::mgr::BufferMgr;
    use file::mgr::FileMgr;
    use log::mgr::LogMgr;
    use tempfile::{TempDir, tempdir};
    use transaction::{lock_table::LockTable, transaction::Transaction};

    pub(crate) fn init() -> (TempDir, Arc<Transaction>) {
        init_with_size(512)
    }

    pub(crate) fn init_with_size(block_size: i32) -> (TempDir, Arc<Transaction>) {
        let dir = tempdir().unwrap();
        let fm = Arc::new(FileMgr::new(dir.path(), block_size).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "testlog".to_string()).unwrap());
        let bm = Arc::new(BufferMgr::new(&fm, &lm, 1).unwrap());
        let lock_table = Arc::new(LockTable::default());

        let tx = Arc::new(Transaction::new(&fm, &lm, &bm, &lock_table).unwrap());
        (dir, tx)
    }
}
