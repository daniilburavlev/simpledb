use std::{path::Path, rc::Rc, sync::Arc};

use buffer::mgr::BufferMgr;
use common::DbResult;
use file::mgr::FileMgr;
use log::mgr::LogMgr;
use transaction::{lock_table::LockTable, transaction::Transaction};

use crate::{
    metadata_mgr::MetadataMgr,
    query::{
        heuristic_planner::{query::HeuristicQueryPlanner, update::HeuristicUpdatePlanner},
        planner::{Planner, QueryPlanner},
    },
    scan::Scan,
};

pub mod buffer_needs;
pub mod element;
pub mod field_info;
pub mod index;
pub mod index_mgr;
pub mod layout;
pub mod metadata_mgr;
pub mod plan;
pub mod predicate;
pub mod query;
pub mod record_page;
pub mod rid;
pub mod scan;
pub mod schema;
mod schema_mapping;
pub mod sort_by;
pub mod stat_mgr;
pub mod table_mgr;
mod temp;
pub mod value;
pub mod view_mgr;

const LOG_FILE: &str = "wal.log";
const BLOCK_SIZE: usize = 8 * 1024;
const NUM_BUFFERS: usize = 1024;

#[derive(Clone)]
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

    pub fn metadata_mgr(&self) -> MetadataMgr {
        self.md.clone()
    }

    pub fn query(&self, tx: &Arc<Transaction>, query: &str) -> DbResult<Rc<dyn Scan>> {
        let planner = self.planner();
        let plan = planner.create_query_plan(query, tx)?;
        plan.open()
    }

    pub fn execute(&self, tx: &Arc<Transaction>, query: &str) -> DbResult<i32> {
        let planner = self.planner();
        planner.execute(query, tx)
    }

    fn planner(&self) -> Planner {
        let query_planner: Rc<dyn QueryPlanner> =
            Rc::new(HeuristicQueryPlanner::new(self.md.clone()));
        let update_planner = HeuristicUpdatePlanner::new(&query_planner, self.md.clone());
        Planner::new(query_planner, Rc::new(update_planner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        element::Element,
        query::{
            analyzer::Analyzer,
            command::{Command, select::ParsedSelectQuery},
            parser::Parser,
        },
    };
    use common::error::DbError;
    use std::collections::{BTreeSet, HashSet};
    use tempfile::{TempDir, tempdir};

    pub(crate) fn init() -> (TempDir, Arc<Transaction>) {
        init_with_size(512)
    }

    pub(crate) fn init_with_size(block_size: i32) -> (TempDir, Arc<Transaction>) {
        let dir = tempdir().unwrap();
        let fm = Arc::new(FileMgr::new(dir.path(), block_size).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "testlog".to_string()).unwrap());
        let bm = Arc::new(BufferMgr::new(&fm, &lm, 16).unwrap());
        let lock_table = Arc::new(LockTable::default());

        let tx = Arc::new(Transaction::new(&fm, &lm, &bm, &lock_table).unwrap());
        (dir, tx)
    }

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
            let id = result.get_i32(&Element::raw("id")).unwrap();
            let name = result.get_string(&Element::raw("name")).unwrap();
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
    fn order_by_field() {
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
            .query(&tx, "SELECT id, name FROM test ORDER BY name")
            .unwrap();
        for i in 0..1000 {
            assert!(result.next().unwrap());
            let id = result.get_i32(&Element::raw("id")).unwrap();
            let name = result.get_string(&Element::raw("name")).unwrap();
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
            let id = result.get_i32(&Element::raw("id")).unwrap();
            assert!(existed.remove(&id));
        }
        assert!(existed.is_empty());
    }

    #[test]
    fn join() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE users(id INT, name VARCHAR(20))")
            .unwrap();
        db.execute(&tx, "CREATE TABLE employees(eid INT, uid INT)")
            .unwrap();
        tx.commit().unwrap();
        for i in 0..100 {
            db.execute(
                &tx,
                &format!("INSERT INTO users(id, name) VALUES({}, 'user{}')", i, i),
            )
            .unwrap();
            db.execute(
                &tx,
                &format!(
                    "INSERT INTO employees(eid, uid) VALUES({}, {})",
                    i + 1000,
                    i
                ),
            )
            .unwrap();
        }
        tx.commit().unwrap();

        let result = db
            .query(
                &tx,
                "SELECT id, name, eid FROM users JOIN employees ON id = uid",
            )
            .unwrap();
        let mut matched = HashSet::new();
        while result.next().unwrap() {
            let id = result.get_i32(&Element::raw("id")).unwrap();
            let eid = result.get_i32(&Element::raw("eid")).unwrap();
            let name = result.get_string(&Element::raw("name")).unwrap();
            assert_eq!(eid, id + 1000);
            assert_eq!(name, format!("user{}", id));
            assert!(matched.insert(id), "each user must match exactly once");
        }
        assert_eq!(matched.len(), 100);
        tx.commit().unwrap();
    }

    #[test]
    fn select_with_views_and_specs() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE test(id INT, name VARCHAR(100))")
            .unwrap();
        tx.commit().unwrap();

        db.execute(&tx, "INSERT INTO test(id, name) VALUES(1, 'User')")
            .unwrap();

        let result = db
            .query(&tx, "SELECT id i, name n FROM test t WHERE t.id=1")
            .unwrap();
        assert!(result.next().unwrap());
        assert_eq!(result.get_i32(&Element::raw("i")).unwrap(), 1);
        assert_eq!(result.get_string(&Element::raw("n")).unwrap(), "User");

        let result = db
            .query(&tx, "SELECT id i, name n FROM test t WHERE t.id=2")
            .unwrap();
        assert!(!result.next().unwrap());
    }

    #[test]
    fn select_with_specs() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE test(id INT, name VARCHAR(100))")
            .unwrap();
        tx.commit().unwrap();

        db.execute(&tx, "INSERT INTO test(id, name) VALUES(1, 'User')")
            .unwrap();

        let result = db
            .query(&tx, "SELECT t.id, t.name FROM test t WHERE t.id=1")
            .unwrap();
        assert!(result.next().unwrap());
        assert_eq!(result.get_i32(&Element::spec("t", "id")).unwrap(), 1);
        assert_eq!(
            result.get_string(&Element::spec("t", "name")).unwrap(),
            "User"
        );
    }

    #[test]
    fn analyze_select_unknown_field() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();

        let tx = db.get_tx().unwrap();
        let md = db.md.clone();

        db.execute(&tx, "CREATE TABLE t1(id INT)").unwrap();
        db.execute(&tx, "CREATE TABLE t2(id INT)").unwrap();

        let parser = Parser::new("SELECT unexisted FROM t1").unwrap();
        let analyzer = Analyzer::new(md.clone(), &tx);
        let query = parsed_query(parser);
        assert!(matches!(
            analyzer.query(query).err().unwrap(),
            DbError::FieldNotExists(field) if field == "unexisted"
        ));

        let parser = Parser::new("SELECT id FROM t1 JOIN t2 ON t1.id = t2.id").unwrap();
        let parsed_query = parsed_query(parser);
        let analyzer = Analyzer::new(md, &tx);
        assert!(matches!(
            analyzer.query(parsed_query).err().unwrap(),
            DbError::Specify(field) if field == "id"
        ));
    }

    #[test]
    fn analyze_select_from_unknown_table() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();

        let tx = db.get_tx().unwrap();
        let md = db.md.clone();

        let parser = Parser::new("SELECT id FROM unknown").unwrap();
        let query = parsed_query(parser);
        let analyzer = Analyzer::new(md, &tx);
        let err = analyzer.query(query).err().unwrap();
        assert!(
            matches!(err, DbError::RelationNotExists(table) if table == "unknown"),
            "table existance not checked"
        );
    }

    #[test]
    fn analyze_select_unspecified_field() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();

        let tx = db.get_tx().unwrap();
        let md = db.md.clone();

        db.execute(&tx, "CREATE TABLE t1(id INT)").unwrap();
        db.execute(&tx, "CREATE TABLE t2(id INT)").unwrap();

        let parser = Parser::new("SELECT id FROM t1 JOIN t2 ON t1.id = t2.id").unwrap();
        let query = parsed_query(parser);
        let analyzer = Analyzer::new(md, &tx);
        assert!(matches!(
            analyzer.query(query).err().unwrap(),
            DbError::Specify(field) if field == "id"
        ));
    }

    #[test]
    fn analyze_select_specified_fields() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();

        let tx = db.get_tx().unwrap();
        let md = db.md.clone();

        db.execute(&tx, "CREATE TABLE t1(id INT)").unwrap();
        db.execute(&tx, "CREATE TABLE t2(id INT)").unwrap();

        let parser = Parser::new("SELECT t1.id, t2.id FROM t1 JOIN t2 ON t1.id = t2.id").unwrap();
        let query = parsed_query(parser);
        let analyzer = Analyzer::new(md, &tx);
        let query = analyzer.query(query).unwrap();

        assert_eq!(
            query.fields,
            vec![Element::spec("t1", "id"), Element::spec("t2", "id")],
            "fields parse error"
        );
        assert_eq!(
            query.tables,
            vec![Element::raw("t1"), Element::raw("t2")],
            "tables parse error"
        );
    }

    #[test]
    fn multiply_inserts() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();

        let tx = db.get_tx().unwrap();
        db.execute(&tx, "CREATE TABLE t(id INT)").unwrap();
        let mut ids: BTreeSet<i32> = (1..10).collect();
        let mut insert = String::from("INSERT INTO t(id) VALUES");
        for id in &ids {
            let id = *id;
            if id == 1 {
                insert.push_str(&format!("({})", id));
            } else {
                insert.push_str(&format!(", ({})", id));
            }
        }
        db.execute(&tx, &insert).unwrap();
        let scan = db.query(&tx, "SELECT id FROM t").unwrap();
        let field = Element::raw("id");
        while scan.next().unwrap() {
            let id = scan.get_i32(&field).unwrap();
            ids.remove(&id);
        }
        assert!(ids.is_empty());
    }

    fn parsed_query(parser: Parser) -> ParsedSelectQuery {
        if let Command::Query(parsed_query) = parser.query().unwrap() {
            parsed_query
        } else {
            panic!("not query");
        }
    }
}
