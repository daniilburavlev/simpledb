use common::DbResult;

use crate::{rid::RID, value::Value};

pub mod b_tree;

pub trait Index {
    fn before_first(&self, key: Value) -> DbResult<()>;

    fn next(&self) -> DbResult<bool>;

    fn get_data_rid(&self) -> DbResult<RID>;

    fn insert(&self, value: Value, rid: RID) -> DbResult<()>;

    fn delete(&self, value: Value, rid: RID) -> DbResult<()>;

    fn close(&self) -> DbResult<()>;
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::schema::SchemaBuilder;
    use crate::{
        SimpleDB,
        plan::{Plan, table::TablePlan},
    };
    use crate::element::Element;

    #[test]
    fn index_retrieval() {
        use crate::schema::Schema;
        use std::sync::Arc;

        let dir = tempdir().unwrap();
        let db = SimpleDB::configured(dir.path(), 512, 8).unwrap();
        let md = db.metadata_mgr();

        let table = "student";
        
        let sid = Element::raw("sid");
        let sname = Element::raw("sname");
        let majorid = Element::raw("majorid");

        let setup_tx = db.get_tx().unwrap();
        let schema = SchemaBuilder::default()
            .add_int_field(sid.clone())
            .add_string_field(sname.clone(), 16)
            .add_int_field(majorid.clone())
            .build();

        md.create_table(table, schema, &setup_tx).unwrap();
        md.create_index("idx_majorid", table, "majorid", &setup_tx)
            .unwrap();
        setup_tx.commit().unwrap();

        let tx = db.get_tx().unwrap();
        let plan = TablePlan::new(&tx, table.to_string(), &md).unwrap();
        let _scan = plan.open().unwrap();

        let indexes = md.get_index_info(table, &tx).unwrap();
        let _index = indexes.get(&majorid).unwrap();
        tx.commit().unwrap();
    }
}
