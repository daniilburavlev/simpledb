use std::sync::Arc;

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{index::Index, rid::RID, value::Value};

pub(crate) struct HashIndex {
    index_name: String,
    position: i32,
    tx: Arc<Transaction>,
    rid: Vec<RID>,
}

impl HashIndex {
    pub(crate) fn new(index_name: &str, tx: &Arc<Transaction>) -> HashIndex {
        HashIndex {
            index_name: index_name.to_string(),
            tx: Arc::clone(tx),
        }
    }
}

impl Index for HashIndex {
    fn before_first(&self, key: Value) -> common::DbResult<()> {
        todo!()
    }

    fn next(&self) -> DbResult<bool> {
        todo!()
    }

    fn get_data_rid(&self) -> DbResult<RID> {
        todo!()
    }

    fn insert(&self, value: Value, rid: RID) -> DbResult<()> {
        todo!()
    }

    fn delete(&self, value: Value, rid: RID) -> DbResult<()> {
        todo!()
    }

    fn close(&self) -> DbResult<()> {
        todo!()
    }
}

fn create_index(tx: &Transaction, index_name: &str) -> DbResult<()> {
    let metadata_block = tx.append(index_name)?;
    let leaf_block = tx.append(index_name)?;

    let metadata = HashPage::Metadata {
        root: leaf_block.num,
    };
    let leaf = HashPage::Leaf {
        parent: 0,
        values: vec![],
        next: -1,
    };
    metadata.write(&metadata_block, tx)?;
    leaf.write(&leaf_block, tx)?;
    Ok(())
}
