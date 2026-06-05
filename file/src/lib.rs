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
