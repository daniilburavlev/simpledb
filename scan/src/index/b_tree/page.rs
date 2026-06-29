use common::{DbResult, error::DbError};
use file::{
    block::BlockId,
    page::{I32_SIZE, Page, U8_SIZE},
};
use transaction::transaction::Transaction;

use crate::{
    index::b_tree::{entry::BTreeEntry, pointer::BTreePointer},
    rid::RID,
    value::{INTEGER_TYPE, VARCHAR_TYPE, Value},
};

pub(crate) const TYPE_SIZE: usize = U8_SIZE;
pub(crate) const POINTER_SIZE: usize = I32_SIZE;
pub(crate) const LEN_SIZE: usize = I32_SIZE;

const METADATA: u8 = 1;
const NODE: u8 = 2;
const LEAF: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BTreePage {
    Metadata {
        root: i32,
    },
    Node {
        parent: i32,
        children: Vec<BTreePointer>,
    },
    Leaf {
        parent: i32,
        values: Vec<BTreeEntry>,
    },
}

impl BTreePage {
    pub(crate) fn read(tx: &Transaction, block: &BlockId) -> DbResult<Self> {
        tx.pin(block)?;
        let mut offset = 0;
        let page_type = tx.get_u8(block, offset)?;
        offset += 1;
        let page = match page_type {
            METADATA => {
                let root = tx.get_i32(block, offset)?;
                Ok(Self::Metadata { root })
            }
            NODE => {
                let parent = tx.get_i32(block, offset)?;
                offset += I32_SIZE;
                let len = tx.get_i32(block, offset)? as usize;
                offset += I32_SIZE;
                let children = read_pointers(tx, block, offset, len)?;
                Ok(Self::Node { parent, children })
            }
            LEAF => {
                let parent = tx.get_i32(block, offset)?;
                offset += I32_SIZE;
                let len = tx.get_i32(block, offset)? as usize;
                offset += I32_SIZE;
                let children = read_entries(tx, block, offset, len)?;
                Ok(Self::Leaf {
                    parent,
                    values: children,
                })
            }
            _ => Err(DbError::other("invalid page type")),
        };
        tx.commit()?;
        page
    }

    pub(crate) fn write(&self, block: &BlockId, tx: &Transaction) -> DbResult<()> {
        tx.pin(block)?;
        match self {
            Self::Metadata { root } => {
                let mut offset = 0;
                tx.set_u8(block, offset, METADATA, true)?;
                offset += 1;
                tx.set_i32(block, offset, *root, true)?;
            }
            Self::Node { parent, children } => {
                let mut offset = 0;
                tx.set_u8(block, offset, NODE, true)?;
                offset += U8_SIZE;
                tx.set_i32(block, offset, *parent, true)?;
                offset += I32_SIZE;
                write_pointers(tx, block, children, offset)?;
            }
            Self::Leaf {
                parent,
                values: children,
            } => {
                let mut offset = 0;
                tx.set_u8(block, offset, LEAF, true)?;
                offset += U8_SIZE;
                tx.set_i32(block, offset, *parent, true)?;
                offset += I32_SIZE;
                write_entries(tx, block, children, offset)?;
            }
        }
        tx.commit()
    }
}

fn read_pointers(
    tx: &Transaction,
    block: &BlockId,
    mut offset: usize,
    len: usize,
) -> DbResult<Vec<BTreePointer>> {
    let mut entries = vec![];
    for _ in 0..len {
        let Some((value, read)) = read_value(tx, block, offset)? else {
            break;
        };
        offset += read;
        let block_num = tx.get_i32(block, offset)?;
        offset += I32_SIZE;
        entries.push(BTreePointer { value, block_num });
    }
    Ok(entries)
}

fn write_pointers(
    tx: &Transaction,
    block: &BlockId,
    children: &[BTreePointer],
    mut offset: usize,
) -> DbResult<()> {
    let len = children.len() as i32;
    tx.set_i32(block, offset, len, true)?;
    offset += POINTER_SIZE;
    for child in children {
        offset += write_value(tx, block, offset, &child.value)?;
        tx.set_i32(block, offset, child.block_num, true)?;
        offset += I32_SIZE;
    }
    Ok(())
}

fn read_entries(
    tx: &Transaction,
    block: &BlockId,
    mut offset: usize,
    len: usize,
) -> DbResult<Vec<BTreeEntry>> {
    let mut entries = vec![];
    for _ in 0..len {
        let (entry, read) = BTreeEntry::read(tx, block, offset)?;
        offset += read;
        entries.push(entry);
    }
    Ok(entries)
}

fn write_entries(
    tx: &Transaction,
    block: &BlockId,
    children: &[BTreeEntry],
    mut offset: usize,
) -> DbResult<()> {
    let len = children.len() as i32;
    tx.set_i32(block, offset, len, true)?;
    offset += LEN_SIZE;
    for child in children {
        let write = child.write(tx, block, offset)?;
        offset += write;
    }
    Ok(())
}

fn read_value(
    tx: &Transaction,
    block: &BlockId,
    mut offset: usize,
) -> DbResult<Option<(Value, usize)>> {
    let value_type = tx.get_u8(block, offset)?;
    offset += 1;
    let mut read = 1;
    let value = match value_type {
        0 => return Ok(None),
        VARCHAR_TYPE => {
            let value = Value::Varchar(tx.get_string(block, offset)?);
            let size = Page::str_space(value.as_str()?);
            read += size;
            value
        }
        INTEGER_TYPE => {
            let value = Value::Integer(tx.get_i32(block, offset)?);
            read += I32_SIZE;
            value
        }
        _ => return Err(DbError::other("invalid index value type")),
    };
    Ok(Some((value, read)))
}

fn write_value(
    tx: &Transaction,
    block: &BlockId,
    mut offset: usize,
    value: &Value,
) -> DbResult<usize> {
    let mut write = 0;
    match value {
        Value::Integer(value) => {
            tx.set_u8(block, offset, INTEGER_TYPE, true)?;
            write += U8_SIZE;
            offset += write;
            tx.set_i32(block, offset, *value, true)?;
            write += I32_SIZE;
        }
        Value::Varchar(value) => {
            let size = file::page::Page::str_space(value);
            tx.set_u8(block, offset, VARCHAR_TYPE, true)?;
            write += U8_SIZE;
            offset += write;
            tx.set_string(block, offset, value, true)?;
            write += size;
        }
    }
    Ok(write)
}

pub(crate) fn pointer_index(values: &[BTreePointer], value: &Value) -> usize {
    values
        .binary_search_by(|k| k.value.cmp(value))
        .unwrap_or_else(|x| if x == 0 { 0 } else { x - 1 })
}

pub(crate) fn insert_pointer(values: &mut Vec<BTreePointer>, value: BTreePointer) {
    let idx = values
        .binary_search_by(|kv| kv.value.cmp(&value.value))
        .unwrap_or_else(|x| x);
    if idx < values.len() && values[idx].value == value.value {
        values[idx] = value;
    } else if idx >= values.len() {
        values.push(value);
    } else {
        values.insert(idx, value);
    }
}

pub(crate) fn insert_entry(values: &mut Vec<BTreeEntry>, key: Value, value: RID) {
    let idx = values
        .binary_search_by(|kv| kv.value.cmp(&key))
        .unwrap_or_else(|x| x);
    if idx < values.len() && values[idx].value == key {
        values[idx].rid.push(value);
    } else if idx >= values.len() {
        values.push(BTreeEntry {
            value: key,
            rid: vec![value],
        });
    } else {
        values.insert(
            idx,
            BTreeEntry {
                value: key,
                rid: vec![value],
            },
        );
    }
}

pub(crate) fn split_pointers(
    mut values: Vec<BTreePointer>,
    block_size: usize,
) -> (Vec<BTreePointer>, Vec<BTreePointer>) {
    let mid = values.len() / 2;
    let mut right = values.split_off(mid);
    let mut size = node_size(&values);
    while size > block_size {
        let value = right.remove(0);
        size -= TYPE_SIZE + value.value.size() + POINTER_SIZE;
        values.push(value);
    }
    (values, right)
}

pub(crate) fn split_entries(
    mut values: Vec<BTreeEntry>,
    block_size: usize,
) -> (Vec<BTreeEntry>, Vec<BTreeEntry>) {
    let mid = values.len() / 2;
    let mut right = values.split_off(mid);
    let mut size = leaf_size(&values);
    while size > block_size {
        let value = right.remove(0);
        size -= TYPE_SIZE + value.value.size() + POINTER_SIZE + POINTER_SIZE;
        values.push(value);
    }
    (values, right)
}

pub(crate) fn node_size(values: &[BTreePointer]) -> usize {
    let mut size = TYPE_SIZE + POINTER_SIZE + LEN_SIZE;
    for value in values {
        size += TYPE_SIZE;
        size += value.value.size();
        size += POINTER_SIZE;
    }
    size
}

pub(crate) fn leaf_size(values: &[BTreeEntry]) -> usize {
    let mut size = TYPE_SIZE + POINTER_SIZE + LEN_SIZE;
    for value in values {
        size += value.size();
    }
    size
}

#[cfg(test)]
mod tests {
    use crate::{rid::RID, tests::init};

    use super::*;

    #[test]
    fn write_read_metadata() {
        let (_dir, tx) = init();

        let metadata = BTreePage::Metadata { root: 11 };
        let block = BlockId::new("test_index", 0);
        metadata.write(&block, &tx).unwrap();
        let restored = BTreePage::read(&tx, &block).unwrap();
        assert_eq!(restored, metadata);
    }

    #[test]
    fn write_read_node() {
        let (_dir, tx) = init();

        let node = BTreePage::Node {
            parent: 1337,
            children: vec![BTreePointer {
                value: Value::Integer(1000),
                block_num: -1337,
            }],
        };
        let block = BlockId::new("test_index", 1);
        node.write(&block, &tx).unwrap();
        let restored = BTreePage::read(&tx, &block).unwrap();
        assert_eq!(restored, node);
    }

    #[test]
    fn write_read_leaf() {
        let (_dir, tx) = init();

        let leaf = BTreePage::Leaf {
            parent: 1337,
            values: vec![BTreeEntry {
                value: Value::Integer(1000),
                rid: vec![RID::new(100, 123)],
            }],
        };
        let block = BlockId::new("test_index", 1);
        leaf.write(&block, &tx).unwrap();
        let restored = BTreePage::read(&tx, &block).unwrap();
        assert_eq!(restored, leaf);
    }
}
