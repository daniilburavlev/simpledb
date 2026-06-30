use std::{collections::HashMap, sync::Arc};

use common::DbResult;
use transaction::transaction::Transaction;

use crate::scan::table::TableScan;
use crate::{
    element::Element,
    field_info::FieldInfo,
    layout::Layout,
    scan::Scan,
    schema::{Schema, SchemaBuilder},
};

const MAX_NAME: i32 = 16;

pub(crate) const TABLE_NAME: &str = "sp_table";
const TABLE_SLOT_SIZE: &str = "slot_size";
const FIELDS_NAME: &str = "sp_fields";

const F_TYPE: &str = "type";
const F_FIELD_NAME: &str = "field";
const F_TYPE_LENGTH: &str = "length";
const F_OFFSET: &str = "offset";

#[derive(Clone, Debug)]
pub struct TableMgr {
    table_layout: Layout,
    fields_layout: Layout,
}

impl TableMgr {
    pub fn new(is_new: bool, tx: &Arc<Transaction>) -> DbResult<Self> {
        let table_schema = SchemaBuilder::default()
            .add_string_field(Element::raw(TABLE_NAME), MAX_NAME)
            .add_int_field(Element::raw(TABLE_SLOT_SIZE))
            .build();
        let table_layout = Layout::new(table_schema.clone());

        let fields_schema = SchemaBuilder::default()
            .add_string_field(Element::raw(TABLE_NAME), MAX_NAME)
            .add_string_field(Element::raw(F_FIELD_NAME), MAX_NAME)
            .add_int_field(Element::raw(F_TYPE))
            .add_int_field(Element::raw(F_TYPE_LENGTH))
            .add_int_field(Element::raw(F_OFFSET))
            .build();
        let fields_layout = Layout::new(fields_schema.clone());

        let mgr = Self {
            table_layout,
            fields_layout,
        };

        if is_new {
            mgr.create_table(TABLE_NAME, table_schema, tx)?;
            mgr.create_table(FIELDS_NAME, fields_schema, tx)?;
        }

        Ok(mgr)
    }

    pub fn create_table(
        &self,
        table_name: &str,
        schema: Schema,
        tx: &Arc<Transaction>,
    ) -> DbResult<()> {
        let layout = Layout::new(schema.clone());

        let tcat = TableScan::new(tx, TABLE_NAME, self.table_layout.clone())?;
        tcat.insert()?;
        tcat.set_string(&Element::raw(TABLE_NAME), table_name)?;
        tcat.set_i32(&Element::raw(TABLE_SLOT_SIZE), layout.slotsize())?;
        tcat.close()?;

        let fcat = TableScan::new(tx, FIELDS_NAME, self.fields_layout.clone())?;
        for (field_name, info) in schema.fields() {
            fcat.insert()?;
            fcat.set_string(&Element::raw(TABLE_NAME), table_name)?;
            fcat.set_string(&Element::raw(F_FIELD_NAME), &field_name.to_string())?;
            fcat.set_i32(&Element::raw(F_TYPE), info.type_id())?;
            fcat.set_i32(&Element::raw(F_TYPE_LENGTH), info.length())?;
            fcat.set_i32(&Element::raw(F_OFFSET), layout.offset(&field_name))?;
        }
        fcat.close()
    }

    pub fn get_layout(&self, table_name: &str, tx: &Arc<Transaction>) -> DbResult<Layout> {
        let mut size = -1;
        let tcat = TableScan::new(tx, TABLE_NAME, self.table_layout.clone())?;
        while tcat.next()? {
            if tcat.get_string(&Element::raw(TABLE_NAME))? == table_name {
                size = tcat.get_i32(&Element::raw(TABLE_SLOT_SIZE))?;
                break;
            }
        }
        tcat.close()?;
        let mut schema = SchemaBuilder::default();
        let mut offsets = HashMap::new();
        let fcat = TableScan::new(tx, FIELDS_NAME, self.fields_layout.clone())?;
        while fcat.next()? {
            let table = fcat.get_string(&Element::raw(TABLE_NAME))?;
            if table == table_name {
                let field_name = fcat.get_string(&Element::raw(F_FIELD_NAME))?;
                let field_type = fcat.get_i32(&Element::raw(F_TYPE))?;
                let field_len = fcat.get_i32(&Element::raw(F_TYPE_LENGTH))?;
                let offset = fcat.get_i32(&Element::raw(F_OFFSET))?;
                schema = schema.add_field(
                    Element::Raw(field_name.clone()),
                    FieldInfo::new(field_type, field_len)?,
                );
                offsets.insert(Element::Raw(field_name), offset);
            }
        }
        let schema = schema.build();
        fcat.close()?;
        Ok(Layout::from(schema, offsets, size))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::tests::init;

    use super::*;

    #[test]
    fn manager() {
        let (_dir, tx) = init();

        let tm = TableMgr::new(true, &tx).unwrap();

        let schema = SchemaBuilder::default()
            .add_int_field(Element::raw("A"))
            .add_string_field(Element::raw("B"), 9)
            .build();

        tm.create_table("MyTable", schema, &tx).unwrap();

        let layout = tm.get_layout("MyTable", &tx).unwrap();
        let mut expected_fields = HashSet::new();
        expected_fields.insert((Element::raw("A"), FieldInfo::Integer));
        expected_fields.insert((Element::raw("B"), FieldInfo::Varchar(9)));
        for (field_name, info) in layout.schema().fields() {
            assert!(expected_fields.contains(&(field_name, info)));
        }
        tx.commit().unwrap();
    }
}
