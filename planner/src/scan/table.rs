use std::sync::Arc;

use common::{DbResult, error::DbError};
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::element::Element;
use crate::schema::Schema;
use crate::value::Value;
use crate::{field_info::FieldInfo, layout::Layout, record_page::RecordPage, rid::RID};

pub(crate) struct TableScan {
    tx: Arc<Transaction>,
    layout: Layout,
    rp: RecordPage,
    filename: String,
    current_slot: i32,
}

impl TableScan {
    pub(crate) fn new(tx: &Arc<Transaction>, table_name: &str, layout: Layout) -> DbResult<Self> {
        let filename = format!("{}.tbl", table_name);
        let rp = if tx.size(&filename)? == 0 {
            let block = tx.append(&filename)?;
            let rp = RecordPage::new(tx, block, layout.clone())?;
            rp.format()?;
            rp
        } else {
            let block = BlockId::new(&filename, 0);
            RecordPage::new(tx, block, layout.clone())?
        };
        Ok(Self {
            tx: Arc::clone(tx),
            layout,
            rp,
            current_slot: -1,
            filename,
        })
    }

    pub(crate) fn close(&self) -> DbResult<()> {
        self.tx.unpin(&self.rp.block())
    }

    pub(crate) fn before_first(&mut self) -> DbResult<()> {
        self.move_to_block(0)
    }

    pub(crate) fn next(&mut self) -> DbResult<bool> {
        self.current_slot = self.rp.next_after(self.current_slot)?;
        while self.current_slot < 0 {
            if self.at_last_block()? {
                return Ok(false);
            }
            self.move_to_block(self.rp.block().num + 1)?;
            self.current_slot = self.rp.next_after(self.current_slot)?;
        }
        Ok(true)
    }

    pub(crate) fn get_i32(&self, filename: &Element) -> DbResult<i32> {
        self.rp.get_i32(self.current_slot, filename)
    }

    pub(crate) fn get_string(&self, filename: &Element) -> DbResult<String> {
        self.rp.get_string(self.current_slot, filename)
    }

    pub(crate) fn get_val(&self, fieldname: &Element) -> DbResult<Value> {
        let Some(info) = self.layout.schema().info(fieldname) else {
            return Err(DbError::FieldNotExists(fieldname.to_string()));
        };
        match info {
            FieldInfo::Integer => Ok(Value::Integer(self.get_i32(fieldname)?)),
            FieldInfo::Varchar(_) => Ok(Value::Varchar(self.get_string(fieldname)?)),
        }
    }

    pub(crate) fn has_field(&self, fieldname: &Element) -> bool {
        self.layout.schema().has_field(fieldname)
    }

    pub(crate) fn set_i32(&self, field: &Element, value: i32) -> DbResult<()> {
        let current_slot = self.current_slot;
        let rp = &self.rp;
        rp.set_i32(current_slot, field, value)
    }

    pub(crate) fn set_string(&self, field: &Element, value: &str) -> DbResult<()> {
        let current_slot = self.current_slot;
        let rp = &self.rp;
        rp.set_string(current_slot, field, value)
    }

    pub(crate) fn set_val(&self, field: &Element, value: Value) -> DbResult<()> {
        match value {
            Value::Integer(value) => self.set_i32(field, value),
            Value::Varchar(value) => self.set_string(field, &value),
        }
    }

    pub(crate) fn insert(&mut self) -> DbResult<()> {
        let last_block = self.tx.size(&self.filename)? as i32 - 1;
        if self.rp.block().num != last_block {
            self.move_to_block(last_block)?;
        }
        self.current_slot = self.rp.insert_after(self.current_slot)?;
        while self.current_slot < 0 {
            if self.at_last_block()? {
                self.move_to_new_block()?;
            } else {
                self.move_to_block(self.rp.block().num + 1)?;
            }
            self.current_slot = self.rp.insert_after(self.current_slot)?;
        }
        Ok(())
    }

    pub(crate) fn delete(&self) -> DbResult<()> {
        self.rp.delete(self.current_slot)
    }

    pub(crate) fn move_to_rid(&mut self, rid: RID) -> DbResult<()> {
        self.close()?;
        let block = BlockId::new(&self.filename, rid.block_num());
        let rp = RecordPage::new(&self.tx, block, self.layout.clone())?;
        self.rp = rp;
        self.current_slot = rid.slot();
        Ok(())
    }

    pub(crate) fn get_rid(&self) -> RID {
        let rp = &self.rp;
        RID::new(rp.block().num, self.current_slot)
    }

    fn move_to_block(&mut self, num: i32) -> DbResult<()> {
        self.close()?;
        let block = BlockId::new(&self.filename, num);
        self.rp = RecordPage::new(&self.tx, block, self.layout.clone())?;
        self.current_slot = -1;
        Ok(())
    }

    fn move_to_new_block(&mut self) -> DbResult<()> {
        self.close()?;
        let block = self.tx.append(&self.filename)?;
        self.rp = RecordPage::new(&self.tx, block, self.layout.clone())?;
        self.rp.format()?;
        self.current_slot = -1;
        Ok(())
    }

    fn at_last_block(&self) -> DbResult<bool> {
        Ok(self.rp.block().num == self.tx.size(&self.filename)? as i32 - 1)
    }

    pub(crate) fn schema(&self) -> &Schema {
        self.layout.schema()
    }
}

#[cfg(test)]
mod tests {
    use crate::{schema::SchemaBuilder, tests::init};

    use super::*;

    #[test]
    fn reopen() {
        let (_dir, tx) = init();
        let layout = Layout::new(SchemaBuilder::default().build());
        let mut scan = TableScan::new(&tx, "test", layout.clone()).unwrap();
        scan.insert().unwrap();
        drop(scan);

        TableScan::new(&tx, "test", layout).unwrap();
    }

    #[test]
    fn move_to_new_block() {
        let (_dir, tx) = init();
        let schema = SchemaBuilder::default()
            .add_string_field(Element::raw("name"), 16)
            .build();
        let layout = Layout::new(schema);
        let mut scan = TableScan::new(&tx, "test", layout).unwrap();
        scan.before_first().unwrap();
        for _ in 0..10 {
            scan.insert().unwrap();
        }
    }
}
