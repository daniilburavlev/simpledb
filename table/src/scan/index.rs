use std::rc::Rc;

use common::DbResult;

use crate::{constant::Constant, index::Index, scan::Scan};

pub struct IndexSelectScan {
    scan: Rc<dyn Scan>,
    index: Rc<dyn Index>,
    value: Constant,
}

impl IndexSelectScan {
    pub fn new(scan: &Rc<dyn Scan>, index: &Rc<dyn Index>, value: Constant) -> DbResult<Self> {
        Ok(Self {
            scan: Rc::clone(scan),
            index: Rc::clone(index),
            value,
        })
    }
}

impl Scan for IndexSelectScan {
    fn before_first(&self) -> DbResult<()> {
        self.index.before_first(self.value.clone())
    }

    fn next(&self) -> DbResult<bool> {
        let ok = self.index.next()?;
        if self.index.next()? {
            let rid = self.index.get_data_rid()?;
            self.scan.move_to_rid(rid)?;
        }
        Ok(ok)
    }

    fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        self.scan.get_i32(field_name)
    }

    fn get_string(&self, field_name: &str) -> DbResult<String> {
        self.scan.get_string(field_name)
    }

    fn get_val(&self, field_name: &str) -> DbResult<Constant> {
        self.scan.get_val(field_name)
    }

    fn has_field(&self, field_name: &str) -> DbResult<bool> {
        self.scan.has_field(field_name)
    }

    fn close(&self) -> DbResult<()> {
        self.index.close()?;
        self.scan.close()
    }

    fn set_i32(&self, _: &str, _: i32) -> DbResult<()> {
        Err(common::error::DbError::other("cannot set integer"))
    }

    fn set_string(&self, _: &str, _: &str) -> DbResult<()> {
        Err(common::error::DbError::other("cannot set string"))
    }

    fn set_val(&self, _: &str, _: Constant) -> DbResult<()> {
        Err(common::error::DbError::other("cannot set value"))
    }

    fn insert(&self) -> DbResult<()> {
        Err(common::error::DbError::other("cannot insert"))
    }

    fn delete(&self) -> DbResult<()> {
        Err(common::error::DbError::other("cannot delete"))
    }

    fn get_rid(&self) -> DbResult<crate::rid::RID> {
        Err(common::error::DbError::other("cannot get rid"))
    }

    fn move_to_rid(&self, _: crate::rid::RID) -> DbResult<()> {
        Err(common::error::DbError::other("cannot update"))
    }
}

pub struct IndexJoinScan {
    left: Rc<dyn Scan>,
    right: Rc<dyn Scan>,
    index: Rc<dyn Index>,
    field: String,
}

impl IndexJoinScan {
    pub fn new(
        left: &Rc<dyn Scan>,
        index: &Rc<dyn Index>,
        field: &str,
        right: &Rc<dyn Scan>,
    ) -> DbResult<Self> {
        let scan = Self {
            left: Rc::clone(left),
            right: Rc::clone(right),
            field: field.to_string(),
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

    fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        if self.right.has_field(field_name)? {
            self.right.get_i32(field_name)
        } else {
            self.left.get_i32(field_name)
        }
    }

    fn get_string(&self, field_name: &str) -> DbResult<String> {
        if self.right.has_field(field_name)? {
            self.right.get_string(field_name)
        } else {
            self.left.get_string(field_name)
        }
    }

    fn get_val(&self, field_name: &str) -> DbResult<Constant> {
        if self.right.has_field(field_name)? {
            self.right.get_val(field_name)
        } else {
            self.left.get_val(field_name)
        }
    }

    fn has_field(&self, field_name: &str) -> DbResult<bool> {
        Ok(self.right.has_field(field_name)? || self.left.has_field(field_name)?)
    }

    fn close(&self) -> DbResult<()> {
        self.left.close()?;
        self.index.close()?;
        self.right.close()
    }
}
