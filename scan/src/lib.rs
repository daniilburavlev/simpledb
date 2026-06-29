use std::sync::Arc;

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    element::Element, layout::Layout, predicate::Predicate, rid::RID, scanner::Scanner,
    schema::Schema, select::SelectScan, table::TableScan, value::Value,
};

pub mod element;
pub mod field_info;
pub mod index;
pub mod layout;
pub mod predicate;
pub mod record_page;
pub mod rid;
pub(crate) mod scanner;
pub mod schema;
pub(crate) mod select;
pub(crate) mod table;
pub mod value;

pub struct Scan {
    scan: Scanner,
}

impl Scan {
    pub fn table(tx: &Arc<Transaction>, table_name: &str, layout: Layout) -> DbResult<Self> {
        let table = TableScan::new(tx, table_name, layout)?;
        Ok(Self {
            scan: Scanner::Table(table),
        })
    }

    pub fn select(scan: Box<Self>, predicate: Predicate) -> Self {
        let select = SelectScan::new(scan, predicate);
        Self {
            scan: Scanner::Select(select),
        }
    }

    pub fn before_first(&mut self) -> DbResult<()> {
        self.scan.before_first()
    }

    pub fn next_row(&mut self) -> DbResult<bool> {
        self.scan.next()
    }

    pub fn get_i32(&self, field: &Element) -> DbResult<i32> {
        self.scan.get_i32(field)
    }

    pub fn get_string(&self, field: &Element) -> DbResult<String> {
        self.scan.get_string(field)
    }

    pub fn get_val(&self, field: &Element) -> DbResult<Value> {
        self.scan.get_val(field)
    }

    pub fn has_field(&self, field: &Element) -> DbResult<bool> {
        self.scan.has_field(field)
    }

    pub fn close(&self) -> DbResult<()> {
        self.scan.close()
    }

    pub fn schema(&self) -> DbResult<Schema> {
        self.scan.schema()
    }

    pub fn set_i32(&self, field: &Element, value: i32) -> DbResult<()> {
        self.scan.set_i32(field, value)
    }

    pub fn set_string(&self, field: &Element, value: &str) -> DbResult<()> {
        self.scan.set_string(field, value)
    }

    pub fn set_val(&self, field: &Element, value: Value) -> DbResult<()> {
        self.scan.set_val(field, value)
    }

    pub fn insert(&mut self) -> DbResult<()> {
        self.scan.insert()
    }

    pub fn delete(&self) -> DbResult<()> {
        self.scan.delete()
    }

    pub fn get_rid(&self) -> DbResult<RID> {
        self.scan.get_rid()
    }

    pub fn move_to_rid(&mut self, rid: RID) -> DbResult<()> {
        self.scan.move_to_rid(rid)
    }

    pub fn save_position(&self) -> DbResult<()> {
        self.scan.save_position()
    }
}

#[cfg(test)]
mod tests {
    use buffer::mgr::BufferMgr;
    use file::mgr::FileMgr;
    use log::mgr::LogMgr;
    use tempfile::{TempDir, tempdir};
    use transaction::lock_table::LockTable;

    use crate::schema::SchemaBuilder;

    use super::*;

    #[test]
    fn scan_table() {
        let (_dir, tx) = init();
        let schema = SchemaBuilder::default()
            .add_int_field(Element::raw("id"))
            .add_string_field(Element::raw("name"), 16)
            .build();
        let layout = Layout::new(schema);
        let mut scan = Scan::table(&tx, "test", layout).unwrap();

        scan.before_first().unwrap();
        assert!(!scan.next_row().unwrap());

        scan.insert().unwrap();
        scan.set_i32(&Element::raw("id"), 1).unwrap();
        scan.set_string(&Element::raw("name"), "hello").unwrap();

        scan.before_first().unwrap();
        assert!(scan.next_row().unwrap());

        assert_eq!(1, scan.get_i32(&Element::raw("id")).unwrap());
        assert_eq!("hello", scan.get_string(&Element::raw("name")).unwrap());

        scan.set_val(&Element::raw("id"), Value::Integer(10))
            .unwrap();
        assert_eq!(
            Value::Integer(10),
            scan.get_val(&Element::raw("id")).unwrap()
        );

        assert!(!scan.has_field(&Element::raw("f")).unwrap());
        scan.schema().unwrap();

        let rid = scan.get_rid().unwrap();
        assert_eq!(rid.slot(), 0);
        assert_eq!(rid.block_num(), 0);

        scan.move_to_rid(RID::new(0, 0)).unwrap();
        scan.before_first().unwrap();
        scan.next_row().unwrap();
        scan.delete().unwrap();
        if let Err(e) = scan.save_position() {
            println!("{}", e);
        } else {
            panic!("save position for table scan not implemented");
        }

        scan.close().unwrap();
    }

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
