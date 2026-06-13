use std::{path::Path, sync::Arc};

use buffer::mgr::BufferMgr;
use common::DbResult;
use file::mgr::FileMgr;
use log::mgr::LogMgr;

pub mod constant;
pub mod field_info;
pub mod index_mgr;
pub mod layout;
pub mod metadata_mgr;
pub mod record_page;
pub mod rid;
pub mod scan;
pub mod schema;
pub mod stat_mgr;
pub mod table_mgr;
pub mod table_scan;
pub mod view_mgr;

const LOG_FILE: &str = "wal";

pub struct SimpleDB {
    fm: Arc<FileMgr>,
    lm: Arc<LogMgr>,
    bm: Arc<BufferMgr>,
}

impl SimpleDB {
    pub fn new(dir: &Path, block_size: usize, num_buffers: usize) -> DbResult<Self> {
        let fm = Arc::new(FileMgr::new(dir, block_size)?);
        let lm = Arc::new(LogMgr::new(&fm, LOG_FILE.to_string())?);
        let bm = Arc::new(BufferMgr::new(&fm, &lm, num_buffers)?);
        Ok(Self { fm, lm, bm })
    }
}
