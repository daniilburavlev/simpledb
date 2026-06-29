use crate::index::b_tree::BTreeIndex;

pub(crate) enum Indexer {
    BTree(BTreeIndex),
}
