use common::DbResult;

use crate::element::Element;
use crate::schema::{Schema, SchemaBuilder};
use crate::value::Value;
use crate::{index::Index, scan::Scan};

pub struct IndexSelectScan {
    scan: Box<Scan>,
    index: Index,
    value: Value,
}

impl IndexSelectScan {
    pub fn new(scan: Box<Scan>, index: Index, value: Value) -> DbResult<Self> {
        let mut scan = Self { scan, index, value };
        scan.before_first()?;
        Ok(scan)
    }

    pub(crate) fn before_first(&mut self) -> DbResult<()> {
        self.index.before_first(self.value.clone())
    }

    pub(crate) fn next_rot(&mut self) -> DbResult<bool> {
        let ok = self.index.next_row()?;
        if ok {
            let rid = self.index.get_data_rid()?;
            self.scan.move_to_rid(rid)?;
        }
        Ok(ok)
    }

    pub(crate) fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        self.scan.get_i32(field_name)
    }

    pub(crate) fn get_string(&self, field_name: &Element) -> DbResult<String> {
        self.scan.get_string(field_name)
    }

    pub(crate) fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        self.scan.get_val(field_name)
    }

    pub(crate) fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        self.scan.has_field(field_name)
    }

    pub(crate) fn close(&self) -> DbResult<()> {
        self.index.close()?;
        self.scan.close()
    }

    pub(crate) fn schema(&self) -> DbResult<Schema> {
        self.scan.schema()
    }
}

pub(crate) struct IndexJoinScan {
    left: Box<Scan>,
    right: Box<Scan>,
    index: Index,
    field: Element,
}

impl IndexJoinScan {
    pub(crate) fn new(
        left: Box<Scan>,
        index: Index,
        field: Element,
        right: Box<Scan>,
    ) -> DbResult<Self> {
        let mut scan = Self {
            left,
            right,
            field,
            index,
        };
        scan.before_first()?;
        Ok(scan)
    }

    pub(crate) fn reset_index(&mut self) -> DbResult<()> {
        let key = self.left.get_val(&self.field)?;
        self.index.before_first(key)
    }

    pub(crate) fn before_first(&mut self) -> DbResult<()> {
        self.left.before_first()?;
        self.left.next_row()?;
        self.reset_index()
    }

    pub(crate) fn next_row(&mut self) -> DbResult<bool> {
        loop {
            if self.index.next_row()? {
                self.right.move_to_rid(self.index.get_data_rid()?)?;
                return Ok(true);
            }
            if !self.left.next_row()? {
                return Ok(false);
            }
            self.reset_index()?;
        }
    }

    pub(crate) fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        if self.right.has_field(field_name)? {
            self.right.get_i32(field_name)
        } else {
            self.left.get_i32(field_name)
        }
    }

    pub(crate) fn get_string(&self, field_name: &Element) -> DbResult<String> {
        if self.right.has_field(field_name)? {
            self.right.get_string(field_name)
        } else {
            self.left.get_string(field_name)
        }
    }

    pub(crate) fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        if self.right.has_field(field_name)? {
            self.right.get_val(field_name)
        } else {
            self.left.get_val(field_name)
        }
    }

    pub(crate) fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        Ok(self.right.has_field(field_name)? || self.left.has_field(field_name)?)
    }

    pub(crate) fn close(&self) -> DbResult<()> {
        self.left.close()?;
        self.index.close()?;
        self.right.close()
    }

    pub(crate) fn schema(&self) -> DbResult<Schema> {
        let s1 = self.left.schema()?;
        let s2 = self.right.schema()?;
        let s = SchemaBuilder::default().add_all(&s1).add_all(&s2).build();
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use crate::element::Element;
    use crate::mgr::metadata::MetadataMgr;
    use crate::plan::table::TablePlan;
    use crate::schema::SchemaBuilder;
    use crate::tests::init;
    use std::collections::HashMap;

    #[test]
    fn update_index() {
        let (_dir, tx) = init();
        let md = MetadataMgr::new(true, &tx).unwrap();
        tx.commit().unwrap();

        let sid = Element::raw("sid");
        let sname = Element::raw("sname");
        let gadyear = Element::raw("gadyear");
        let majorid = Element::raw("majorid");

        let schema = SchemaBuilder::default()
            .add_int_field(sid.clone())
            .add_string_field(sname.clone(), 16)
            .add_int_field(gadyear.clone())
            .add_int_field(majorid.clone())
            .build();

        md.create_table("users", schema, &tx).unwrap();
        md.create_index("users_ids", "users", "sid", &tx).unwrap();

        let plan = TablePlan::new(&tx, "users".to_string(), &md).unwrap();
        let mut s = plan.open().unwrap();

        let mut indexes = HashMap::new();
        let infos = md.get_index_info("users", &tx).unwrap();
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
        while s.next_row().unwrap() {
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
        while s.next_row().unwrap() {
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
