use std::sync::Arc;

use common::DbResult;
use transaction::transaction::Transaction;

use crate::{
    index::{b_tree::BTreeIndex, indexer::Indexer},
    rid::RID,
    value::Value,
};

pub(crate) mod b_tree;
mod indexer;

pub struct Index(Indexer);

impl Index {
    pub fn new(index_name: &str, tx: &Arc<Transaction>) -> DbResult<Self> {
        let index = BTreeIndex::new(index_name, tx)?;
        Ok(Self(Indexer::BTree(index)))
    }

    pub fn before_first(&mut self, key: Value) -> DbResult<()> {
        match &mut self.0 {
            Indexer::BTree(index) => index.before_first(key),
        }
    }

    pub fn next_row(&mut self) -> DbResult<bool> {
        match &mut self.0 {
            Indexer::BTree(index) => index.next(),
        }
    }

    pub fn get_data_rid(&self) -> DbResult<RID> {
        match &self.0 {
            Indexer::BTree(index) => index.get_data_rid(),
        }
    }

    pub fn insert(&self, value: Value, rid: RID) -> DbResult<()> {
        match &self.0 {
            Indexer::BTree(index) => index.insert(value, rid),
        }
    }

    pub fn delete(&self, value: Value, rid: RID) -> DbResult<()> {
        match &self.0 {
            Indexer::BTree(index) => index.delete(value, rid),
        }
    }

    pub fn close(&self) -> DbResult<()> {
        match &self.0 {
            Indexer::BTree(index) => index.close(),
        }
    }
}
