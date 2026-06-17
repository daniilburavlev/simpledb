use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use common::DbResult;

use crate::open_file;

pub(crate) struct FileHolder {
    pub(crate) dir: PathBuf,
    open_files: HashMap<String, File>,
}

impl FileHolder {
    pub(crate) fn new(path: &Path) -> Self {
        let dir = PathBuf::from(path);
        Self {
            dir,
            open_files: HashMap::new(),
        }
    }

    pub(crate) fn get(&mut self, filename: &str) -> DbResult<&File> {
        let path = self.dir.join(filename);
        let file = self
            .open_files
            .entry(filename.to_string())
            .or_insert(open_file(&path)?);
        Ok(file)
    }
}
