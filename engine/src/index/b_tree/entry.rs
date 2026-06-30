use crate::value::{INTEGER_TYPE, VARCHAR_TYPE};
use crate::{rid::RID, value::Value};
use common::DbResult;
use common::error::DbError;
use file::block::BlockId;
use file::page::{I32_SIZE, Page, U8_SIZE};
use std::cmp::Ordering;
use transaction::transaction::Transaction;

const TYPE_SIZE: usize = U8_SIZE;
const LEN_SIZE: usize = I32_SIZE;
const PTR_SIZE: usize = I32_SIZE;

#[derive(Clone, Debug)]
pub(crate) struct BTreeEntry {
    pub(crate) value: Value,
    pub(crate) rid: Vec<RID>,
}

impl BTreeEntry {
    pub(crate) fn read(
        tx: &Transaction,
        block: &BlockId,
        offset: usize,
    ) -> DbResult<(Self, usize)> {
        let mut read = 0;
        let value_type = tx.get_u8(block, offset)?;
        read += TYPE_SIZE;
        let value = match value_type {
            VARCHAR_TYPE => Value::Varchar(tx.get_string(block, offset + read)?),
            INTEGER_TYPE => Value::Integer(tx.get_i32(block, offset + read)?),
            _ => return Err(DbError::other("unexpected value type")),
        };
        read += value.size();
        let mut rid = vec![];
        let len = tx.get_i32(block, offset + read)?;
        read += LEN_SIZE;
        for _ in 0..len {
            let block_num = tx.get_i32(block, offset + read)?;
            read += I32_SIZE;
            let slot = tx.get_i32(block, offset + read)?;
            read += I32_SIZE;
            rid.push(RID::new(block_num, slot));
        }
        Ok((Self { value, rid }, read))
    }

    pub(crate) fn write(
        &self,
        tx: &Transaction,
        block: &BlockId,
        offset: usize,
    ) -> DbResult<usize> {
        let mut write = 0;
        match &self.value {
            Value::Integer(value) => {
                tx.set_u8(block, offset, INTEGER_TYPE, true)?;
                write += TYPE_SIZE;
                tx.set_i32(block, offset + write, *value, true)?;
                write += I32_SIZE;
            }
            Value::Varchar(value) => {
                tx.set_u8(block, offset, VARCHAR_TYPE, true)?;
                write += TYPE_SIZE;
                tx.set_string(block, offset + write, value, true)?;
                write += Page::str_space(value);
            }
        }
        let len = self.rid.len() as i32;
        tx.set_i32(block, offset + write, len, true)?;
        write += LEN_SIZE;
        for r in &self.rid {
            tx.set_i32(block, offset + write, r.block_num(), true)?;
            write += I32_SIZE;
            tx.set_i32(block, offset + write, r.slot(), true)?;
            write += I32_SIZE;
        }
        Ok(write)
    }

    pub(crate) fn size(&self) -> usize {
        let mut size = TYPE_SIZE + self.value.size() + LEN_SIZE;
        for _ in &self.rid {
            size += 2 * PTR_SIZE;
        }
        size
    }
}

impl PartialEq for BTreeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for BTreeEntry {}

impl PartialOrd for BTreeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BTreeEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::init;

    #[test]
    fn write_read_entry() {
        let (_dir, tx) = init();
        let block = BlockId::new("test", 0);
        tx.pin(&block).unwrap();

        let entry = BTreeEntry {
            value: Value::Integer(10),
            rid: vec![RID::new(1, 0), RID::new(2, 10), RID::new(3, 100)],
        };
        entry.write(&tx, &block, 0).unwrap();
        let restored = BTreeEntry::read(&tx, &block, 0).unwrap();

        assert_eq!(restored.0.value, entry.value);
        assert_eq!(restored.0.rid, entry.rid);
    }
}
