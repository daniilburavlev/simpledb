use common::DbResult;
use std::rc::Rc;

use crate::element::Element;
use crate::schema::{Schema, SchemaBuilder};
use crate::{index::Index, scan::Scan, value::Value};

pub struct IndexSelectScan {
    scan: Rc<dyn Scan>,
    index: Rc<dyn Index>,
    value: Value,
}

impl IndexSelectScan {
    pub fn new(scan: &Rc<dyn Scan>, index: &Rc<dyn Index>, value: Value) -> DbResult<Self> {
        let scan = Self {
            scan: Rc::clone(scan),
            index: Rc::clone(index),
            value,
        };
        scan.before_first()?;
        Ok(scan)
    }
}

impl Scan for IndexSelectScan {
    fn before_first(&self) -> DbResult<()> {
        self.index.before_first(self.value.clone())
    }

    fn next(&self) -> DbResult<bool> {
        let ok = self.index.next()?;
        if ok {
            let rid = self.index.get_data_rid()?;
            self.scan.move_to_rid(rid)?;
        }
        Ok(ok)
    }

    fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        self.scan.get_i32(field_name)
    }

    fn get_string(&self, field_name: &Element) -> DbResult<String> {
        self.scan.get_string(field_name)
    }

    fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        self.scan.get_val(field_name)
    }

    fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        self.scan.has_field(field_name)
    }

    fn close(&self) -> DbResult<()> {
        self.index.close()?;
        self.scan.close()
    }

    fn schema(&self) -> DbResult<Schema> {
        self.scan.schema()
    }
}

pub struct IndexJoinScan {
    left: Rc<dyn Scan>,
    right: Rc<dyn Scan>,
    index: Rc<dyn Index>,
    field: Element,
}

impl IndexJoinScan {
    pub fn new(
        left: &Rc<dyn Scan>,
        index: &Rc<dyn Index>,
        field: Element,
        right: &Rc<dyn Scan>,
    ) -> DbResult<Self> {
        let scan = Self {
            left: Rc::clone(left),
            right: Rc::clone(right),
            field,
            index: Rc::clone(index),
        };
        scan.before_first()?;
        Ok(scan)
    }

    fn reset_index(&self) -> DbResult<()> {
        let key = self.left.get_val(&self.field)?;
        self.index.before_first(key)
    }
}

impl Scan for IndexJoinScan {
    fn before_first(&self) -> DbResult<()> {
        self.left.before_first()?;
        self.left.next()?;
        self.reset_index()
    }

    fn next(&self) -> DbResult<bool> {
        loop {
            if self.index.next()? {
                self.right.move_to_rid(self.index.get_data_rid()?)?;
                return Ok(true);
            }
            if !self.left.next()? {
                return Ok(false);
            }
            self.reset_index()?;
        }
    }

    fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        if self.right.has_field(field_name)? {
            self.right.get_i32(field_name)
        } else {
            self.left.get_i32(field_name)
        }
    }

    fn get_string(&self, field_name: &Element) -> DbResult<String> {
        if self.right.has_field(field_name)? {
            self.right.get_string(field_name)
        } else {
            self.left.get_string(field_name)
        }
    }

    fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        if self.right.has_field(field_name)? {
            self.right.get_val(field_name)
        } else {
            self.left.get_val(field_name)
        }
    }

    fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        Ok(self.right.has_field(field_name)? || self.left.has_field(field_name)?)
    }

    fn close(&self) -> DbResult<()> {
        self.left.close()?;
        self.index.close()?;
        self.right.close()
    }

    fn schema(&self) -> DbResult<Schema> {
        let s1 = self.left.schema()?;
        let s2 = self.right.schema()?;
        let s = SchemaBuilder::new(Element::Raw(format!(
            "index_joni_{}_{}",
            s1.table(),
            s2.table()
        )))
        .add_all(&s1)
        .add_all(&s2)
        .build();
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use tempfile::tempdir;

    use crate::element::Element;
    use crate::schema::SchemaBuilder;
    use crate::{
        SimpleDB,
        plan::{Plan, table::TablePlan},
    };

    #[test]
    fn index_scan_update() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();
        let tx = db.get_tx().unwrap();
        let md = db.metadata_mgr();

        let sid = Element::raw("sid");
        let sname = Element::raw("sname");
        let gadyear = Element::raw("gadyear");
        let majorid = Element::raw("majorid");

        let table = "users";

        let schema = SchemaBuilder::new(Element::raw(table))
            .add_int_field(sid.clone())
            .add_string_field(sname.clone(), 16)
            .add_int_field(gadyear.clone())
            .add_int_field(majorid.clone())
            .build();

        md.create_table(table, schema, &tx).unwrap();
        md.create_index("users_ids", table, "sid", &tx).unwrap();

        let plan = TablePlan::new(&tx, table.to_string(), &md).unwrap();
        let s = plan.open().unwrap();

        let mut indexes = HashMap::new();
        let infos = md.get_index_info(table, &tx).unwrap();
        for (field, info) in infos {
            let index = info.open().unwrap();
            indexes.insert(field, index);
        }

        s.insert().unwrap();
        s.set_i32(&sid, 11).unwrap();
        s.set_string(&sname, "Sam").unwrap();
        s.set_i32(&gadyear, 2023).unwrap();
        s.set_i32(&majorid, 30).unwrap();

        let rid = s.get_rid().unwrap();
        for (field, index) in indexes.iter() {
            let value = s.get_val(field).unwrap();
            index.insert(value, rid.clone()).unwrap();
        }

        s.before_first().unwrap();
        while s.next().unwrap() {
            if s.get_string(&sname).unwrap() == "joe" {
                let rid = s.get_rid().unwrap();
                for (field, index) in indexes.iter() {
                    let value = s.get_val(field).unwrap();
                    index.delete(value, rid.clone()).unwrap();
                }
                s.delete().unwrap();
                break;
            }
        }
        s.before_first().unwrap();
        while s.next().unwrap() {
            println!(
                "{} {}",
                s.get_string(&sname).unwrap(),
                s.get_i32(&sid).unwrap()
            );
        }
        s.close().unwrap();
        for idx in indexes.values() {
            idx.close().unwrap();
        }
        tx.commit().unwrap();
    }
}
