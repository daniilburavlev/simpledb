use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    path::{Path, PathBuf},
};

use common::DbResult;

use crate::open_file;

/// Upper bound on the number of file handles kept open at once. External merge
/// sort spills each run to a uniquely-named temp table, so without a cap the
/// open handles accumulate until the OS rejects new ones with "Too many open
/// files". Evicted files are transparently reopened on the next access.
const MAX_OPEN_FILES: usize = 100;

pub(crate) struct FileHolder {
    pub(crate) dir: PathBuf,
    open_files: HashMap<String, File>,
    /// Least-recently-used filename at the front, most-recently-used at the back.
    lru: VecDeque<String>,
}

impl FileHolder {
    pub(crate) fn new(path: &Path) -> Self {
        let dir = PathBuf::from(path);
        Self {
            dir,
            open_files: HashMap::new(),
            lru: VecDeque::new(),
        }
    }

    pub(crate) fn get(&mut self, filename: &str) -> DbResult<&File> {
        if !self.open_files.contains_key(filename) {
            self.evict_if_needed();
            let path = self.dir.join(filename);
            self.open_files
                .insert(filename.to_string(), open_file(&path)?);
        }
        self.touch(filename);
        Ok(self.open_files.get(filename).expect("just inserted"))
    }

    /// Mark `filename` as most-recently-used.
    fn touch(&mut self, filename: &str) {
        if let Some(pos) = self.lru.iter().position(|f| f == filename) {
            self.lru.remove(pos);
        }
        self.lru.push_back(filename.to_string());
    }

    /// Close the least-recently-used handle if we are at capacity.
    fn evict_if_needed(&mut self) {
        while self.open_files.len() >= MAX_OPEN_FILES {
            match self.lru.pop_front() {
                Some(victim) => {
                    self.open_files.remove(&victim);
                }
                None => break,
            }
        }
    }
}
