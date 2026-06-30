use std::{path::Path, rc::Rc, sync::Arc};

use buffer::mgr::BufferMgr;
use common::DbResult;
use file::mgr::FileMgr;
use log::mgr::LogMgr;
use planner::mgr::metadata::MetadataMgr;
use planner::scan::Scan;
use transaction::{lock_table::LockTable, transaction::Transaction};

use crate::query::{
    basic_planner::BasicUpdatePlanner, heuristic_planner::HeuristicQueryPlanner, planner::Planner,
};

pub mod query;
mod order;

const LOG_FILE: &str = "wal.log";
const BLOCK_SIZE: usize = 8 * 1024;
const NUM_BUFFERS: usize = 1024;

pub struct SimpleDB {
    fm: Arc<FileMgr>,
    lm: Arc<LogMgr>,
    bm: Arc<BufferMgr>,
    lock_table: Arc<LockTable>,
    md: MetadataMgr,
}

impl SimpleDB {
    pub fn new(dir: &Path) -> DbResult<Self> {
        Self::configured(dir, BLOCK_SIZE, NUM_BUFFERS)
    }

    pub fn configured(dir: &Path, block_size: usize, num_buffers: usize) -> DbResult<Self> {
        let fm = Arc::new(FileMgr::new(dir, block_size.try_into().unwrap())?);
        let lm = Arc::new(LogMgr::new(&fm, LOG_FILE.to_string())?);
        let bm = Arc::new(BufferMgr::new(&fm, &lm, num_buffers)?);
        let lock_table = Arc::new(LockTable::default());
        let tx = Arc::new(Transaction::new(&fm, &lm, &bm, &lock_table)?);
        let is_new = fm.is_new()?;
        if is_new {
            tracing::debug!("creating new database");
        } else {
            tracing::debug!("recovering existing database");
            tx.recover()?;
        }
        let md = MetadataMgr::new(is_new, &tx)?;
        tx.commit()?;
        Ok(Self {
            fm,
            lm,
            bm,
            lock_table,
            md,
        })
    }

    pub fn get_tx(&self) -> DbResult<Arc<Transaction>> {
        let tx = Transaction::new(&self.fm, &self.lm, &self.bm, &self.lock_table)?;
        Ok(Arc::new(tx))
    }

    pub fn metadata_mgr(&self) -> &MetadataMgr {
        &self.md
    }

    pub fn query(&self, tx: &Arc<Transaction>, query: &str) -> DbResult<Scan> {
        let planner = self.planner();
        let plan = planner.create_query_plan(query, tx)?;
        plan.open()
    }

    pub fn execute(&self, tx: &Arc<Transaction>, query: &str) -> DbResult<i32> {
        let planner = self.planner();
        planner.execute_update(query, tx)
    }

    fn planner(&self) -> Planner {
        let query_planner = HeuristicQueryPlanner::new(&self.md);
        let update_planner = BasicUpdatePlanner::new(&self.md);
        Planner::new(Rc::new(query_planner), Rc::new(update_planner))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn select_with_index() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE users(id INT, name VARCHAR(16))")
            .unwrap();
        db.execute(&tx, "CREATE INDEX user_ids ON users(id)")
            .unwrap();
        db.execute(&tx, "INSERT INTO users(id, name) VALUES(1, 'User User')")
            .unwrap();
        db.execute(&tx, "INSERT INTO users(id, name) VALUES(2, 'Name')")
            .unwrap();
        let result = db
            .query(&tx, "SELECT id, name FROM users WHERE id = 2")
            .unwrap();
        while result.next().unwrap() {
            let id = result.get_i32("id").unwrap();
            let name = result.get_string("name").unwrap();
            assert_eq!(id, 2);
            assert_eq!(name, "Name");
        }
    }

    #[test]
    fn group_by() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE test(id INT)").unwrap();
        for i in 0..10 {
            for _ in 0..100 {
                db.execute(&tx, &format!("INSERT INTO test(id) VALUES({})", i))
                    .unwrap();
            }
        }
        let result = db.query(&tx, "SELECT id FROM test").unwrap();
        for _ in 0..1000 {
            let existed = result.next().unwrap();
            assert!(existed);
        }
        let result = db.query(&tx, "SELECT id FROM test GROUP BY id").unwrap();
        for _ in 0..10 {
            let existed = result.next().unwrap();
            assert!(existed);
        }
    }

    #[test]
    fn sort_by() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE test(id INT, name VARCHAR(10))")
            .unwrap();
        let names = ["a", "b", "c", "d", "e"];
        let mut ids = HashSet::new();
        for i in 0..1000 {
            ids.insert(i);
            db.execute(
                &tx,
                &format!(
                    "INSERT INTO test(id, name) VALUES({}, '{}')",
                    i,
                    names[i % names.len()]
                ),
            )
            .unwrap();
        }
        let result = db
            .query(&tx, "SELECT id, name FROM test SORT BY name")
            .unwrap();
        for i in 0..1000 {
            assert!(result.next().unwrap());
            let id = result.get_i32("id").unwrap();
            let name = result.get_string("name").unwrap();
            assert!(ids.remove(&(id as usize)));
            if i < 200 {
                assert_eq!(name, "a");
            } else if i < 400 {
                assert_eq!(name, "b");
            } else if i < 600 {
                assert_eq!(name, "c");
            } else if i < 800 {
                assert_eq!(name, "d");
            } else {
                assert_eq!(name, "e");
            }
        }
        assert!(!result.next().unwrap());
        assert!(ids.is_empty());
    }

    #[test]
    fn insert_10000_rows() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE test(id INT)").unwrap();
        tx.commit().unwrap();

        let mut existed = HashSet::new();
        for i in 0..10000 {
            db.execute(&tx, &format!("INSERT INTO test(id) VALUES({})", i))
                .unwrap();
            existed.insert(i);
        }
        tx.commit().unwrap();

        let result = db.query(&tx, "SELECT id FROM test").unwrap();
        while result.next().unwrap() {
            let id = result.get_i32("id").unwrap();
            assert!(existed.remove(&id));
        }
        assert!(existed.is_empty());
    }

    #[test]
    fn join() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE t1(i1 INT)").unwrap();
        db.execute(&tx, "CREATE TABLE t2(i2 INT)").unwrap();
        tx.commit().unwrap();
        for i in 0..100 {
            db.execute(&tx, &format!("INSERT INTO t1(i1) VALUES({})", i))
                .unwrap();
            db.execute(&tx, &format!("INSERT INTO t2(i2) VALUES({})", i))
                .unwrap();
        }
        tx.commit().unwrap();
        let mut existed = HashSet::new();
        for i in 0..10 {
            existed.insert(i);
            let result = db
                .query(
                    &tx,
                    &format!("SELECT i1, i2 FROM t1, t2 WHERE i1 = i2 AND i1 = {}", i),
                )
                .unwrap();
            while result.next().unwrap() {
                assert_eq!(i, result.get_i32("i1").unwrap());
                assert_eq!(i, result.get_i32("i2").unwrap());
            }
        }
        tx.commit().unwrap();
    }
}
