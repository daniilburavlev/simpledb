use common::DbResult;

use crate::{constant::Constant, rid::RID};

pub mod b_tree;
pub mod btree_dir;
pub mod btree_leaf;
pub mod btree_page;
pub mod dir_entry;

pub trait Index {
    fn before_first(&self, key: Constant) -> DbResult<()>;

    fn next(&self) -> DbResult<bool>;

    fn get_data_rid(&self) -> DbResult<RID>;

    fn insert(&self, value: Constant, rid: RID) -> DbResult<()>;

    fn delete(&self, value: Constant, rid: RID) -> DbResult<()>;

    fn close(&self) -> DbResult<()>;
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::{
        SimpleDB,
        plan::{Plan, table::TablePlan},
    };

    #[test]
    fn index_retrieval() {
        use crate::schema::Schema;
        use std::sync::Arc;

        let dir = tempdir().unwrap();
        let db = SimpleDB::configured(dir.path(), 512, 8).unwrap();
        let md = db.metadata_mgr();

        let table = "student";

        let setup_tx = db.get_tx().unwrap();
        let schema = Arc::new(Schema::default());
        schema.add_int_field("sid".to_string()).unwrap();
        schema.add_string_field("sname".to_string(), 16).unwrap();
        schema.add_int_field("majorid".to_string()).unwrap();
        md.create_table(table, &schema, &setup_tx).unwrap();
        md.create_index("idx_majorid", table, "majorid", &setup_tx)
            .unwrap();
        setup_tx.commit().unwrap();

        let tx = db.get_tx().unwrap();
        let plan = TablePlan::new(&tx, table.to_string(), &md).unwrap();
        let _scan = plan.open().unwrap();

        let indexes = md.get_index_info(table, &tx).unwrap();
        let _index = indexes.get("majorid").unwrap();
        tx.commit().unwrap();
    }
}
