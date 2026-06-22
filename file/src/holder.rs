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
        if !self.open_files.contains_key(filename) {
            let path = self.dir.join(filename);
            self.open_files
                .insert(filename.to_string(), open_file(&path)?);
        }
        Ok(self.open_files.get(filename).expect("just inserted"))
    }
}
