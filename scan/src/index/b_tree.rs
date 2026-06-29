use std::sync::Arc;

use common::{DbResult, error::DbError};
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::{
    index::b_tree::{
        page::{
            BTreePage, POINTER_SIZE, TYPE_SIZE, insert_entry, insert_pointer, leaf_size, node_size,
            pointer_index, split_entries, split_pointers,
        },
        pointer::BTreePointer,
    },
    rid::RID,
    value::Value,
};

pub(crate) mod entry;
pub(crate) mod page;
pub(crate) mod pointer;

pub(crate) struct BTreeIndex {
    index_name: String,
    tx: Arc<Transaction>,
    position: i32,
    rid: Vec<RID>,
}

impl BTreeIndex {
    pub(crate) fn new(index_name: &str, tx: &Arc<Transaction>) -> DbResult<Self> {
        if tx.size(index_name)? == 0 {
            create_index(tx, index_name)?;
        }
        Ok(Self {
            index_name: index_name.to_string(),
            tx: Arc::clone(tx),
            position: 0,
            rid: vec![],
        })
    }

    pub(crate) fn before_first(&mut self, key: Value) -> DbResult<()> {
        let mut block = BlockId::new(&self.index_name, 0);
        let mut page = BTreePage::read(&self.tx, &block)?;
        loop {
            match page {
                BTreePage::Metadata { root } => {
                    block = BlockId::new(&self.index_name, root);
                    page = BTreePage::read(&self.tx, &block)?;
                }
                BTreePage::Node { children, .. } => {
                    let idx = pointer_index(&children, &key);
                    block = BlockId::new(&self.index_name, children[idx].block_num);
                    page = BTreePage::read(&self.tx, &block)?;
                }
                BTreePage::Leaf { values, .. } => {
                    match values.binary_search_by(|v| v.value.cmp(&key)) {
                        Ok(idx) => {
                            self.position = -1;
                            self.rid = values[idx].rid.clone();
                        }
                        Err(_) => tracing::debug!("value not found"),
                    };
                    return Ok(());
                }
            }
        }
    }

    pub(crate) fn next(&mut self) -> DbResult<bool> {
        self.position += 1;
        Ok((self.position as usize) < self.rid.len())
    }

    pub(crate) fn get_data_rid(&self) -> DbResult<RID> {
        Ok(self.rid[self.position as usize].clone())
    }

    pub(crate) fn insert(&self, key: Value, rid: RID) -> DbResult<()> {
        let mut block = BlockId::new(&self.index_name, 0);
        let mut page = BTreePage::read(&self.tx, &block)?;
        let mut new_offset = None::<(Value, i32)>;
        let block_size = self.tx.block_size() as usize;
        let mut new_root = None::<i32>;

        loop {
            match page {
                BTreePage::Metadata { root } => {
                    if let Some(root) = new_root.take() {
                        let page = BTreePage::Metadata { root };
                        page.write(&block, &self.tx)?;
                        break;
                    }
                    block = BlockId::new(&self.index_name, root);
                    page = BTreePage::read(&self.tx, &block)?;
                }
                BTreePage::Node {
                    parent,
                    mut children,
                } => {
                    if let Some((key, offset)) = new_offset.take() {
                        insert_pointer(
                            &mut children,
                            BTreePointer {
                                value: key,
                                block_num: offset,
                            },
                        );
                        if node_size(&children) <= block_size {
                            let page = BTreePage::Node { parent, children };
                            page.write(&block, &self.tx)?;
                            break;
                        }
                        let (children, right_children) = split_pointers(children, block_size);
                        let left_key = children[0].value.clone();
                        let right_key = right_children[0].value.clone();
                        if parent == 0 {
                            let parent_block = self.tx.append(&self.index_name)?;
                            new_root = Some(parent_block.num);
                            let right_block = self.tx.append(&self.index_name)?;
                            let left = BTreePage::Node {
                                parent: parent_block.num,
                                children,
                            };
                            self.rewrite_parent(&right_children, parent_block.num)?;
                            let right = BTreePage::Node {
                                parent: parent_block.num,
                                children: right_children,
                            };
                            let parent = BTreePage::Node {
                                parent: 0,
                                children: vec![
                                    BTreePointer {
                                        value: left_key,
                                        block_num: block.num,
                                    },
                                    BTreePointer {
                                        value: right_key,
                                        block_num: right_block.num,
                                    },
                                ],
                            };
                            parent.write(&parent_block, &self.tx)?;
                            left.write(&block, &self.tx)?;
                            right.write(&right_block, &self.tx)?;
                            block = BlockId::new(&self.index_name, 0);
                            page = BTreePage::Metadata {
                                root: parent_block.num,
                            };
                        } else {
                            block = BlockId::new(&self.index_name, parent);
                            page = BTreePage::read(&self.tx, &block)?;
                            let right = BTreePage::Node {
                                parent,
                                children: right_children.clone(),
                            };
                            let right_block = self.tx.append(&self.index_name)?;
                            right.write(&right_block, &self.tx)?;
                            new_offset = Some((right_key, right_block.num));
                        }
                    } else {
                        let idx = pointer_index(&children, &key);
                        let Some(child_offset) = children.get(idx) else {
                            return Err(DbError::other("empty node's leafs"));
                        };
                        block = BlockId::new(&self.index_name, child_offset.block_num);
                        page = BTreePage::read(&self.tx, &block)?;
                    }
                }
                BTreePage::Leaf {
                    parent,
                    values: mut children,
                } => {
                    let block_size = self.tx.block_size() as usize;
                    let size = TYPE_SIZE + key.size();
                    let max_size = block_size - (TYPE_SIZE + 3 * POINTER_SIZE);
                    if size > max_size {
                        return Err(DbError::MaxSize(max_size, size));
                    }
                    insert_entry(&mut children, key.clone(), rid.clone());
                    if leaf_size(&children) <= block_size {
                        let page = BTreePage::Leaf {
                            parent,
                            values: children,
                        };
                        page.write(&block, &self.tx)?;
                        break;
                    }
                    let (children, right_children) = split_entries(children, block_size);
                    let left_key = children[0].value.clone();
                    let right_key = right_children[0].value.clone();
                    if parent == 0 {
                        let parent_block = self.tx.append(&self.index_name)?;
                        let right_block = self.tx.append(&self.index_name)?;
                        let left = BTreePage::Leaf {
                            parent: parent_block.num,
                            values: children,
                        };
                        let right = BTreePage::Leaf {
                            parent: parent_block.num,
                            values: right_children,
                        };
                        let parent = BTreePage::Node {
                            parent: 0,
                            children: vec![
                                BTreePointer {
                                    value: left_key,
                                    block_num: block.num,
                                },
                                BTreePointer {
                                    value: right_key,
                                    block_num: right_block.num,
                                },
                            ],
                        };
                        parent.write(&parent_block, &self.tx)?;
                        left.write(&block, &self.tx)?;
                        right.write(&right_block, &self.tx)?;
                        new_root = Some(parent_block.num);
                        block = BlockId::new(&self.index_name, 0);
                        page = BTreePage::Metadata {
                            root: parent_block.num,
                        };
                    } else {
                        let left = BTreePage::Leaf {
                            parent,
                            values: children,
                        };
                        let right_block = self.tx.append(&self.index_name)?;
                        let right = BTreePage::Leaf {
                            parent,
                            values: right_children,
                        };
                        left.write(&block, &self.tx)?;
                        right.write(&right_block, &self.tx)?;
                        new_offset = Some((right_key, right_block.num));
                        block = BlockId::new(&self.index_name, parent);
                        page = BTreePage::read(&self.tx, &block)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn delete(&self, key: Value, rid: RID) -> DbResult<()> {
        let mut block = BlockId::new(&self.index_name, 0);
        let mut page = BTreePage::read(&self.tx, &block)?;
        loop {
            match page {
                BTreePage::Metadata { root } => {
                    block = BlockId::new(&self.index_name, root);
                    page = BTreePage::read(&self.tx, &block)?;
                }
                BTreePage::Node { children, .. } => {
                    let idx = pointer_index(&children, &key);
                    block = BlockId::new(&self.index_name, children[idx].block_num);
                    page = BTreePage::read(&self.tx, &block)?;
                }
                BTreePage::Leaf { parent, mut values } => {
                    if let Ok(idx) = values.binary_search_by(|v| v.value.cmp(&key))
                        && let Some(position) = values[idx].rid.iter().position(|x| *x == rid)
                    {
                        values[idx].rid.remove(position);
                        let page = BTreePage::Leaf { parent, values };
                        page.write(&block, &self.tx)?;
                    }
                    break;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn close(&self) -> DbResult<()> {
        todo!()
    }

    fn rewrite_parent(&self, values: &[BTreePointer], parent: i32) -> DbResult<()> {
        for value in values {
            let block = BlockId::new(&self.index_name, value.block_num);
            match BTreePage::read(&self.tx, &block)? {
                BTreePage::Node { children, .. } => {
                    let page = BTreePage::Node { parent, children };
                    page.write(&block, &self.tx)?;
                }
                BTreePage::Leaf { values, .. } => {
                    let page = BTreePage::Leaf { parent, values };
                    page.write(&block, &self.tx)?;
                }
                _ => return Err(DbError::other("unexpected B-Tree index page type")),
            }
        }
        Ok(())
    }
}

fn create_index(tx: &Transaction, index_name: &str) -> DbResult<()> {
    let metadata_block = tx.append(index_name)?;
    let leaf_block = tx.append(index_name)?;

    let metadata = BTreePage::Metadata {
        root: leaf_block.num,
    };
    let leaf = BTreePage::Leaf {
        parent: 0,
        values: vec![],
    };
    metadata.write(&metadata_block, tx)?;
    leaf.write(&leaf_block, tx)?;
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{init, init_with_size};

    #[test]
    fn insert_1000() {
        let (_dir, tx) = init();
        let mut index = BTreeIndex::new("test_index", &tx).unwrap();
        for i in 0..1000 {
            index.insert(Value::Integer(i), RID::new(i, i)).unwrap();
        }
        for i in 0..1000 {
            index.before_first(Value::Integer(i)).unwrap();
            assert!(index.next().unwrap());
        }
        tx.commit().unwrap();
    }

    #[test]
    fn two_leafs_one_node() {
        let (_dir, tx) = init();
        let index_name = "test";
        let index = BTreeIndex::new("test", &tx).unwrap();
        for i in 0..30 {
            index.insert(Value::Integer(i), RID::new(i, i)).unwrap();
        }
        let metadata_block = BlockId::new(index_name, 0);
        let left_block = BlockId::new(index_name, 1);
        let root_block = BlockId::new(index_name, 2);
        let right_block = BlockId::new(index_name, 3);

        if let BTreePage::Metadata { root } = BTreePage::read(&tx, &metadata_block).unwrap() {
            assert_eq!(root, 2);
        } else {
            panic!("expected metadata page");
        }
        if let BTreePage::Leaf { parent, values } = BTreePage::read(&tx, &left_block).unwrap() {
            assert_eq!(parent, 2);
            assert_eq!(values.len(), 15);
        } else {
            panic!("expected left leaf page");
        }
        if let BTreePage::Node { parent, children } = BTreePage::read(&tx, &root_block).unwrap() {
            assert_eq!(parent, 0);
            assert_eq!(children.len(), 2);
        } else {
            panic!("expected root node page");
        }
        if let BTreePage::Leaf { parent, values } = BTreePage::read(&tx, &right_block).unwrap() {
            assert_eq!(parent, 2);
            assert_eq!(values.len(), 15);
        } else {
            panic!("expected right leaf page");
        }
        tx.commit().unwrap();
    }

    #[test]
    fn small_page_test() {
        let (_dir, tx) = init_with_size(32);
        let mut index = BTreeIndex::new("test_index", &tx).unwrap();
        for i in 0..10 {
            index.insert(Value::Integer(i), RID::new(i, i)).unwrap();
        }
        for i in 0..10 {
            index.before_first(Value::Integer(i)).unwrap();
            assert!(index.next().unwrap());
        }
        tx.commit().unwrap();
    }
}
