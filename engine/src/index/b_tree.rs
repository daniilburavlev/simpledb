use std::cell::RefCell;
use std::sync::Arc;

use common::{DbResult, error::DbError};
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::index::Index;
use crate::index::b_tree::entry::{BTreeEntry, OVERFLOW_SIZE};
use crate::index::b_tree::page::{LEN_SIZE, leaf_header_size, overflow_size};
use crate::{
    index::b_tree::{
        page::{
            BTreePage, POINTER_SIZE, TYPE_SIZE, insert_pointer, leaf_size, node_size,
            pointer_index, split_entries, split_pointers, update_pointer,
        },
        pointer::BTreePointer,
    },
    rid::RID,
    value::Value,
};

mod entry;
mod page;
mod pointer;

struct BTreeIndexInner {
    index_name: String,
    tx: Arc<Transaction>,
    position: i32,
    rid: Vec<RID>,
}

impl BTreeIndexInner {
    fn new(index_name: &str, tx: &Arc<Transaction>) -> DbResult<Self> {
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

    fn before_first(&mut self, key: Value) -> DbResult<()> {
        let mut block = BlockId::new(&self.index_name, 0);
        let mut page = BTreePage::read(&self.tx, &block)?;
        loop {
            match page {
                BTreePage::Metadata { root, .. } => {
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
                            self.rid = self.collect_rids(&values[idx])?;
                        }
                        Err(_) => tracing::debug!("value not found"),
                    };
                    return Ok(());
                }
                BTreePage::Overflow { .. } => {
                    return Err(DbError::other("unexpected overflow page during descent"));
                }
            }
        }
    }

    fn collect_rids(&self, entry: &BTreeEntry) -> DbResult<Vec<RID>> {
        let mut rids = entry.rid.clone();
        let mut next = entry.overflow;
        while next != -1 {
            let block = BlockId::new(&self.index_name, next);
            match BTreePage::read(&self.tx, &block)? {
                BTreePage::Overflow {
                    rids: chunk,
                    next: chunk_next,
                } => {
                    rids.extend(chunk);
                    next = chunk_next;
                }
                _ => return Err(DbError::other("expected overflow page")),
            }
        }
        Ok(rids)
    }

    fn next(&mut self) -> DbResult<bool> {
        self.position += 1;
        Ok((self.position as usize) < self.rid.len())
    }

    fn get_data_rid(&self) -> DbResult<RID> {
        Ok(self.rid[self.position as usize])
    }

    fn insert(&self, key: Value, rid: RID) -> DbResult<()> {
        let mut block = BlockId::new(&self.index_name, 0);
        let mut page = BTreePage::read(&self.tx, &block)?;
        // Pending split to fold into the parent: (left_key, left_block, right_key, right_block).
        let mut split = None::<(Value, i32, Value, i32)>;
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
                    next,
                } => {
                    if let Some((left_key, left_block, right_key, right_block)) = split.take() {
                        // Refresh the split child's separator (its min may have
                        // dropped) and add the separator for its new right half.
                        update_pointer(&mut children, left_block, left_key);
                        insert_pointer(
                            &mut children,
                            BTreePointer {
                                value: right_key,
                                block_num: right_block,
                            },
                        );
                        if node_size(&children) <= block_size {
                            let page = BTreePage::Node {
                                parent,
                                children,
                                next,
                            };
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
                            self.rewrite_parent(&right_children, right_block.num)?;
                            let left = BTreePage::Node {
                                parent: parent_block.num,
                                children,
                                next: right_block.num,
                            };
                            let right = BTreePage::Node {
                                parent: parent_block.num,
                                children: right_children,
                                next,
                            };
                            let root = BTreePage::Node {
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
                                next: -1,
                            };
                            root.write(&parent_block, &self.tx)?;
                            left.write(&block, &self.tx)?;
                            right.write(&right_block, &self.tx)?;
                            block = BlockId::new(&self.index_name, 0);
                            page = BTreePage::Metadata {
                                root: parent_block.num,
                            };
                        } else {
                            let right_block = self.tx.append(&self.index_name)?;
                            self.rewrite_parent(&right_children, right_block.num)?;
                            let left = BTreePage::Node {
                                parent,
                                children,
                                next: right_block.num,
                            };
                            let right = BTreePage::Node {
                                parent,
                                children: right_children,
                                next,
                            };
                            left.write(&block, &self.tx)?;
                            right.write(&right_block, &self.tx)?;
                            split = Some((left_key, block.num, right_key, right_block.num));
                            block = BlockId::new(&self.index_name, parent);
                            page = BTreePage::read(&self.tx, &block)?;
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
                    next,
                } => {
                    let entry_budget = block_size - leaf_header_size();
                    let min_entry = TYPE_SIZE + key.size() + OVERFLOW_SIZE + LEN_SIZE;
                    if min_entry > entry_budget {
                        return Err(DbError::MaxSize(entry_budget, min_entry));
                    }
                    self.leaf_insert_rid(&mut children, key.clone(), rid, entry_budget)?;
                    if leaf_size(&children) <= block_size {
                        let page = BTreePage::Leaf {
                            parent,
                            values: children,
                            next,
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
                            next: right_block.num,
                        };
                        let right = BTreePage::Leaf {
                            parent: parent_block.num,
                            values: right_children,
                            next,
                        };
                        let root = BTreePage::Node {
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
                            next: -1,
                        };
                        root.write(&parent_block, &self.tx)?;
                        left.write(&block, &self.tx)?;
                        right.write(&right_block, &self.tx)?;
                        new_root = Some(parent_block.num);
                        block = BlockId::new(&self.index_name, 0);
                        page = BTreePage::Metadata {
                            root: parent_block.num,
                        };
                    } else {
                        let right_block = self.tx.append(&self.index_name)?;
                        let left = BTreePage::Leaf {
                            parent,
                            values: children,
                            next: right_block.num,
                        };
                        let right = BTreePage::Leaf {
                            parent,
                            values: right_children,
                            next,
                        };
                        left.write(&block, &self.tx)?;
                        right.write(&right_block, &self.tx)?;
                        split = Some((left_key, block.num, right_key, right_block.num));
                        block = BlockId::new(&self.index_name, parent);
                        page = BTreePage::read(&self.tx, &block)?;
                    }
                }
                BTreePage::Overflow { .. } => {
                    return Err(DbError::other("unexpected overflow page during insert"));
                }
            }
        }
        Ok(())
    }

    fn leaf_insert_rid(
        &self,
        children: &mut Vec<BTreeEntry>,
        key: Value,
        rid: RID,
        entry_budget: usize,
    ) -> DbResult<()> {
        match children.binary_search_by(|kv| kv.value.cmp(&key)) {
            Ok(idx) => {
                if children[idx].size() + 2 * POINTER_SIZE <= entry_budget {
                    children[idx].rid.push(rid);
                } else {
                    let head = self.append_overflow(children[idx].overflow, rid)?;
                    children[idx].overflow = head;
                }
            }
            Err(idx) => {
                let base = TYPE_SIZE + key.size() + OVERFLOW_SIZE + LEN_SIZE;
                let entry = if base + 2 * POINTER_SIZE <= entry_budget {
                    BTreeEntry {
                        value: key,
                        rid: vec![rid],
                        overflow: -1,
                    }
                } else {
                    let head = self.append_overflow(-1, rid)?;
                    BTreeEntry {
                        value: key,
                        rid: vec![],
                        overflow: head,
                    }
                };
                children.insert(idx, entry);
            }
        }
        Ok(())
    }

    fn append_overflow(&self, head: i32, rid: RID) -> DbResult<i32> {
        let block_size = self.tx.block_size() as usize;
        if head != -1 {
            let block = BlockId::new(&self.index_name, head);
            if let BTreePage::Overflow { mut rids, next } = BTreePage::read(&self.tx, &block)?
                && overflow_size(rids.len() + 1) <= block_size
            {
                rids.push(rid);
                BTreePage::Overflow { rids, next }.write(&block, &self.tx)?;
                return Ok(head);
            }
        }
        let new_block = self.tx.append(&self.index_name)?;
        BTreePage::Overflow {
            rids: vec![rid],
            next: head,
        }
        .write(&new_block, &self.tx)?;
        Ok(new_block.num)
    }

    fn delete(&self, key: Value, rid: RID) -> DbResult<()> {
        let mut block = BlockId::new(&self.index_name, 0);
        let mut page = BTreePage::read(&self.tx, &block)?;
        loop {
            match page {
                BTreePage::Metadata { root, .. } => {
                    block = BlockId::new(&self.index_name, root);
                    page = BTreePage::read(&self.tx, &block)?;
                }
                BTreePage::Node { children, .. } => {
                    let idx = pointer_index(&children, &key);
                    block = BlockId::new(&self.index_name, children[idx].block_num);
                    page = BTreePage::read(&self.tx, &block)?;
                }
                BTreePage::Leaf {
                    parent,
                    mut values,
                    next,
                } => {
                    if let Ok(idx) = values.binary_search_by(|v| v.value.cmp(&key)) {
                        if let Some(position) = values[idx].rid.iter().position(|x| *x == rid) {
                            values[idx].rid.remove(position);
                            let page = BTreePage::Leaf {
                                parent,
                                values,
                                next,
                            };
                            page.write(&block, &self.tx)?;
                        } else {
                            self.delete_from_overflow(values[idx].overflow, &rid)?;
                        }
                    }
                    break;
                }
                BTreePage::Overflow { .. } => {
                    return Err(DbError::other("unexpected overflow page during delete"));
                }
            }
        }
        Ok(())
    }

    fn delete_from_overflow(&self, head: i32, rid: &RID) -> DbResult<()> {
        let mut next = head;
        while next != -1 {
            let block = BlockId::new(&self.index_name, next);
            let BTreePage::Overflow {
                mut rids,
                next: chunk_next,
            } = BTreePage::read(&self.tx, &block)?
            else {
                return Err(DbError::other("expected overflow page"));
            };
            if let Some(position) = rids.iter().position(|x| x == rid) {
                rids.remove(position);
                BTreePage::Overflow {
                    rids,
                    next: chunk_next,
                }
                .write(&block, &self.tx)?;
                return Ok(());
            }
            next = chunk_next;
        }
        Ok(())
    }

    fn close(&self) -> DbResult<()> {
        Ok(())
    }

    fn rewrite_parent(&self, values: &[BTreePointer], parent: i32) -> DbResult<()> {
        for value in values {
            let block = BlockId::new(&self.index_name, value.block_num);
            match BTreePage::read(&self.tx, &block)? {
                BTreePage::Node { children, next, .. } => {
                    let page = BTreePage::Node {
                        parent,
                        children,
                        next,
                    };
                    page.write(&block, &self.tx)?;
                }
                BTreePage::Leaf { values, next, .. } => {
                    let page = BTreePage::Leaf {
                        parent,
                        values,
                        next,
                    };
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
        next: -1,
    };
    metadata.write(&metadata_block, tx)?;
    leaf.write(&leaf_block, tx)?;
    Ok(())
}

pub(crate) struct BTreeIndex(RefCell<BTreeIndexInner>);

impl BTreeIndex {
    pub(crate) fn new(index_name: &str, tx: &Arc<Transaction>) -> DbResult<Self> {
        Ok(Self(RefCell::new(BTreeIndexInner::new(index_name, tx)?)))
    }
}

impl Index for BTreeIndex {
    fn before_first(&self, key: Value) -> DbResult<()> {
        let mut inner = self.0.borrow_mut();
        inner.before_first(key)
    }

    fn next(&self) -> DbResult<bool> {
        let mut inner = self.0.borrow_mut();
        inner.next()
    }

    fn get_data_rid(&self) -> DbResult<RID> {
        let inner = self.0.borrow();
        inner.get_data_rid()
    }

    fn insert(&self, key: Value, rid: RID) -> DbResult<()> {
        let inner = self.0.borrow();
        inner.insert(key, rid)
    }

    fn delete(&self, key: Value, rid: RID) -> DbResult<()> {
        let inner = self.0.borrow();
        inner.delete(key, rid)
    }

    fn close(&self) -> DbResult<()> {
        let inner = self.0.borrow();
        inner.close()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{init, init_with_size};

    #[test]
    fn insert_1000() {
        let (_dir, tx) = init();
        let mut index = BTreeIndexInner::new("test_index", &tx).unwrap();
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
    fn shuffled_insert_small_page() {
        let (_dir, tx) = init_with_size(48);
        let mut index = BTreeIndexInner::new("shuf", &tx).unwrap();
        // Non-monotonic insertion order forces splits of non-leftmost leaves and
        // inserts of keys below a leaf's current minimum — the case that a stale
        // left separator (fixed via update_pointer) used to orphan.
        let order = [5, 2, 8, 1, 9, 3, 7, 0, 6, 4];
        for &i in &order {
            index.insert(Value::Integer(i), RID::new(i, i)).unwrap();
        }
        for i in 0..10 {
            index.before_first(Value::Integer(i)).unwrap();
            assert!(index.next().unwrap(), "key {i} not found");
            assert_eq!(index.get_data_rid().unwrap(), RID::new(i, i));
        }
        tx.commit().unwrap();
    }

    #[test]
    fn descending_insert() {
        // Pure descending order: every insert goes to the leftmost leaf and
        // repeatedly pushes its minimum down.
        let (_dir, tx) = init_with_size(64);
        let mut index = BTreeIndexInner::new("desc", &tx).unwrap();
        for i in (0..200).rev() {
            index.insert(Value::Integer(i), RID::new(i, i)).unwrap();
        }
        for i in 0..200 {
            index.before_first(Value::Integer(i)).unwrap();
            assert!(index.next().unwrap(), "key {i} not found");
        }
        tx.commit().unwrap();
    }

    #[test]
    fn two_leafs_one_node() {
        let (_dir, tx) = init();
        let index_name = "test";
        let index = BTreeIndexInner::new("test", &tx).unwrap();
        for i in 0..30 {
            index.insert(Value::Integer(i), RID::new(i, i)).unwrap();
        }
        let metadata_block = BlockId::new(index_name, 0);
        let left_block = BlockId::new(index_name, 1);
        let root_block = BlockId::new(index_name, 2);
        let right_block = BlockId::new(index_name, 3);

        let left_len;
        let right_len;

        if let BTreePage::Metadata { root } = BTreePage::read(&tx, &metadata_block).unwrap() {
            assert_eq!(root, 2);
        } else {
            panic!("expected metadata page");
        }
        if let BTreePage::Leaf {
            parent,
            values,
            next,
        } = BTreePage::read(&tx, &left_block).unwrap()
        {
            assert_eq!(parent, 2);
            assert!(!values.is_empty());
            assert_eq!(next, right_block.num);
            left_len = values.len();
        } else {
            panic!("expected left leaf page");
        }
        if let BTreePage::Node {
            parent,
            children,
            next,
        } = BTreePage::read(&tx, &root_block).unwrap()
        {
            assert_eq!(parent, 0);
            assert_eq!(children.len(), 2);
            assert_eq!(-1, next);
        } else {
            panic!("expected root node page");
        }
        if let BTreePage::Leaf {
            parent,
            values,
            next,
        } = BTreePage::read(&tx, &right_block).unwrap()
        {
            assert_eq!(parent, 2);
            assert!(!values.is_empty());
            assert_eq!(next, -1);
            right_len = values.len();
        } else {
            panic!("expected right leaf page");
        }
        assert_eq!(left_len + right_len, 30);
        tx.commit().unwrap();
    }

    #[test]
    fn small_page_test() {
        let (_dir, tx) = init_with_size(32);
        let mut index = BTreeIndexInner::new("test_index", &tx).unwrap();
        for i in 0..10 {
            index.insert(Value::Integer(i), RID::new(i, i)).unwrap();
        }
        for i in 0..10 {
            index.before_first(Value::Integer(i)).unwrap();
            assert!(index.next().unwrap());
        }
        tx.commit().unwrap();
    }

    #[test]
    fn duplicate_key_spills_to_overflow_pages() {
        let (_dir, tx) = init_with_size(64);
        let mut index = BTreeIndexInner::new("test_index", &tx).unwrap();

        let n = 50;
        for i in 0..n {
            index.insert(Value::Integer(7), RID::new(i, i)).unwrap();
        }
        for i in 0..5 {
            index
                .insert(Value::Integer(100 + i), RID::new(i, i))
                .unwrap();
        }

        index.before_first(Value::Integer(7)).unwrap();
        let mut seen = std::collections::HashSet::new();
        while index.next().unwrap() {
            seen.insert(index.get_data_rid().unwrap());
        }
        assert_eq!(seen.len(), n as usize);
        for i in 0..n {
            assert!(seen.contains(&RID::new(i, i)));
        }

        assert!(tx.size("test_index").unwrap() > 4);
        tx.commit().unwrap();
    }

    #[test]
    fn delete_removes_rid_from_overflow_chain() {
        let (_dir, tx) = init_with_size(64);
        let mut index = BTreeIndexInner::new("test_index", &tx).unwrap();
        let n = 40;
        for i in 0..n {
            index.insert(Value::Integer(7), RID::new(i, i)).unwrap();
        }
        index.delete(Value::Integer(7), RID::new(30, 30)).unwrap();
        index.before_first(Value::Integer(7)).unwrap();
        let mut seen = std::collections::HashSet::new();
        while index.next().unwrap() {
            seen.insert(index.get_data_rid().unwrap());
        }
        assert_eq!(seen.len(), (n - 1) as usize);
        assert!(!seen.contains(&RID::new(30, 30)));
        tx.commit().unwrap();
    }
}
