use common::DbResult;

use crate::{index::indexer::Indexer, rid::RID, value::Value};

pub(crate) mod b_tree;
mod indexer;

pub struct Index(Indexer);

impl Index {
    pub fn before_first(&mut self, key: Value) -> DbResult<()> {
        match &mut self.0 {
            Indexer::BTree(index) => index.before_first(key),
        }
    }

    pub fn next(&mut self) -> DbResult<bool> {
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
