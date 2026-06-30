use std::{collections::HashMap, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    field_info::FieldInfo,
    layout::Layout,
    scan::{Scan, table::TableScan},
    schema::Schema,
};

const MAX_NAME: i32 = 16;

pub(crate) const TABLE_NAME: &str = "sp_table";
const TABLE_SLOT_SIZE: &str = "slot_size";
const FIELDS_NAME: &str = "sp_fields";

const F_TYPE: &str = "type";
const F_FIELD_NAME: &str = "field";
const F_TYPE_LENGTH: &str = "length";
const F_OFFSET: &str = "offset";

pub struct TableMgr {
    table_catalog_layout: Arc<Layout>,
    fields_catalog_layout: Arc<Layout>,
}

impl TableMgr {
    pub fn new(is_new: bool, tx: &Arc<Transaction>) -> DbResult<Self> {
        let table_catalog_schema = Arc::new(Schema::default());
        table_catalog_schema.add_string_field(TABLE_NAME.to_string(), MAX_NAME)?;
        table_catalog_schema.add_int_field(TABLE_SLOT_SIZE.to_string())?;
        let table_catalog_layout = Layout::new(&table_catalog_schema)?;

        let fields_catalog_schema = Arc::new(Schema::default());
        fields_catalog_schema.add_string_field(TABLE_NAME.to_string(), MAX_NAME)?;
        fields_catalog_schema.add_string_field(F_FIELD_NAME.to_string(), MAX_NAME)?;
        fields_catalog_schema.add_int_field(F_TYPE.to_string())?;
        fields_catalog_schema.add_int_field(F_TYPE_LENGTH.to_string())?;
        fields_catalog_schema.add_int_field(F_OFFSET.to_string())?;
        let fields_catalog_layout = Layout::new(&fields_catalog_schema)?;

        let mgr = Self {
            table_catalog_layout: Arc::new(table_catalog_layout),
            fields_catalog_layout: Arc::new(fields_catalog_layout),
        };

        if is_new {
            mgr.create_table(TABLE_NAME, &table_catalog_schema, tx)?;
            mgr.create_table(FIELDS_NAME, &fields_catalog_schema, tx)?;
        }

        Ok(mgr)
    }

    pub fn create_table(
        &self,
        table_name: &str,
        schema: &Arc<Schema>,
        tx: &Arc<Transaction>,
    ) -> DbResult<()> {
        let layout = Layout::new(schema)?;

        let tcat = TableScan::new(tx, TABLE_NAME, &self.table_catalog_layout)?;
        tcat.insert()?;
        tcat.set_string(TABLE_NAME, table_name)?;
        tcat.set_i32(TABLE_SLOT_SIZE, layout.slotsize())?;
        tcat.close()?;

        let fcat = TableScan::new(tx, FIELDS_NAME, &self.fields_catalog_layout)?;
        for (field_name, info) in schema.fields()? {
            fcat.insert()?;
            fcat.set_string(TABLE_NAME, table_name)?;
            fcat.set_string(F_FIELD_NAME, &field_name)?;
            fcat.set_i32(F_TYPE, info.type_id())?;
            fcat.set_i32(F_TYPE_LENGTH, info.length())?;
            fcat.set_i32(F_OFFSET, layout.offset(&field_name))?;
        }
        fcat.close()
    }

    pub fn get_layout(&self, table_name: &str, tx: &Arc<Transaction>) -> DbResult<Layout> {
        let mut size = -1;
        let tcat = TableScan::new(tx, TABLE_NAME, &self.table_catalog_layout)?;
        while tcat.next()? {
            if tcat.get_string(TABLE_NAME)? == table_name {
                size = tcat.get_i32(TABLE_SLOT_SIZE)?;
                break;
            }
        }
        tcat.close()?;
        let schema = Arc::new(Schema::default());
        let mut offsets = HashMap::new();
        let fcat = TableScan::new(tx, FIELDS_NAME, &self.fields_catalog_layout)?;
        while fcat.next()? {
            let table = fcat.get_string(TABLE_NAME)?;
            if table == table_name {
                let field_name = fcat.get_string(F_FIELD_NAME)?;
                let field_type = fcat.get_i32(F_TYPE)?;
                let field_len = fcat.get_i32(F_TYPE_LENGTH)?;
                let offset = fcat.get_i32(F_OFFSET)?;
                schema.add_field(field_name.clone(), FieldInfo::new(field_type, field_len)?)?;
                offsets.insert(field_name, offset);
            }
        }
        fcat.close()?;
        Ok(Layout::from(&schema, offsets, size))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::SimpleDB;
    use tempfile::tempdir;

    #[test]
    fn manager() {
        let dir = tempdir().unwrap();
        let db = SimpleDB::new(dir.path()).unwrap();

        let tx = db.get_tx().unwrap();
        let tm = TableMgr::new(true, &tx).unwrap();

        let schema = Arc::new(Schema::default());
        schema.add_int_field("A".to_string()).unwrap();
        schema.add_string_field("B".to_string(), 9).unwrap();

        tm.create_table("MyTable", &schema, &tx).unwrap();

        let layout = tm.get_layout("MyTable", &tx).unwrap();
        let mut expected_fields = HashSet::new();
        expected_fields.insert(("A".to_string(), FieldInfo::Integer));
        expected_fields.insert(("B".to_string(), FieldInfo::Varchar(9)));
        for (field_name, info) in layout.schema().fields().unwrap() {
            assert!(expected_fields.contains(&(field_name, info)));
        }
        tx.commit().unwrap();
    }
}
