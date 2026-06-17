use std::sync::{Arc, RwLock};

use common::{DbResult, error::DbError};
use file::block::BlockId;
use transaction::transaction::Transaction;

use crate::{
    constant::Constant,
    field_info::FieldInfo,
    index::{
        Index,
        btree_dir::BTreeDir,
        btree_leaf::BTreeLeaf,
        btree_page::{BLOCK, BTreePage, VALUE},
    },
    layout::Layout,
    schema::Schema,
};

struct BTreeIndexLock {
    tx: Arc<Transaction>,
    dir_layout: Arc<Layout>,
    leaf_layout: Arc<Layout>,
    leaf_table: String,
    leaf: Option<BTreeLeaf>,
    root_block: BlockId,
}

impl BTreeIndexLock {
    fn new(tx: &Arc<Transaction>, name: &str, leaf_layout: &Arc<Layout>) -> DbResult<Self> {
        let leaf_table = format!("{}_leaf", name);
        if tx.size(&leaf_table)? == 0 {
            let block = tx.append(&leaf_table)?;
            let node = BTreePage::new(tx, block.clone(), leaf_layout)?;
            node.format(&block, -1)?;
        }
        let dir_schema = Arc::new(Schema::default());
        dir_schema.add(BLOCK.to_string(), &leaf_layout.schema())?;
        dir_schema.add(VALUE.to_string(), &leaf_layout.schema())?;
        let dir_table = format!("{}_dir", name);
        let dir_layout = Arc::new(Layout::new(&dir_schema)?);
        let root_block = BlockId::new(&dir_table, 0);
        if tx.size(&dir_table)? == 0 {
            tx.append(&dir_table)?;
            let node = BTreePage::new(tx, root_block.clone(), &dir_layout)?;
            node.format(&root_block, 0)?;
            if let Some(field) = dir_schema.info(VALUE)? {
                let min = match field {
                    FieldInfo::Integer => Constant::Integer(i32::MIN),
                    FieldInfo::Varchar(_) => Constant::Varchar("".to_string()),
                };
                node.insert_dir(0, min, 0)?;
            }
            node.close()?;
        }
        Ok(Self {
            tx: Arc::clone(tx),
            dir_layout,
            leaf_layout: Arc::clone(leaf_layout),
            leaf_table,
            leaf: None,
            root_block,
        })
    }

    fn before_first(&mut self, key: Constant) -> DbResult<()> {
        self.close()?;
        let root = BTreeDir::new(&self.tx, self.root_block.clone(), &self.dir_layout)?;
        let block_num = root.search(&key)?;
        root.close()?;
        let leaf_block = BlockId::new(&self.leaf_table, block_num);
        self.leaf = Some(BTreeLeaf::new(
            &self.tx,
            leaf_block,
            &self.leaf_layout,
            key,
        )?);
        Ok(())
    }

    fn next(&self) -> DbResult<bool> {
        if let Some(leaf) = &self.leaf {
            leaf.next()
        } else {
            Ok(false)
        }
    }

    fn get_data_rid(&self) -> DbResult<crate::rid::RID> {
        if let Some(leaf) = &self.leaf {
            leaf.get_data_rid()
        } else {
            Err(DbError::other("cannot find RID"))
        }
    }

    fn insert(&mut self, value: Constant, rid: crate::rid::RID) -> DbResult<()> {
        self.before_first(value)?;
        let Some(leaf) = &self.leaf else {
            return Ok(());
        };
        let Some(e) = leaf.insert(rid)? else {
            return Ok(());
        };
        let root = BTreeDir::new(&self.tx, self.root_block.clone(), &self.dir_layout)?;
        if let Some(e2) = root.insert(e)? {
            root.make_new_root(e2)?;
        }
        root.close()?;
        Ok(())
    }

    fn delete(&mut self, value: Constant, rid: crate::rid::RID) -> DbResult<()> {
        self.before_first(value.clone())?;
        if let Some(leaf) = &self.leaf {
            leaf.delete(rid)?;
        }
        Ok(())
    }

    fn close(&self) -> DbResult<()> {
        if let Some(leaf) = &self.leaf {
            leaf.close()?;
        }
        Ok(())
    }
}

pub struct BTreeIndex {
    lock: RwLock<BTreeIndexLock>,
}

impl BTreeIndex {
    pub fn new(tx: &Arc<Transaction>, name: &str, leaf_layout: &Arc<Layout>) -> DbResult<Self> {
        Ok(Self {
            lock: RwLock::new(BTreeIndexLock::new(tx, name, leaf_layout)?),
        })
    }
}

impl Index for BTreeIndex {
    fn before_first(&self, key: Constant) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.before_first(key)
    }

    fn next(&self) -> DbResult<bool> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.next()
    }

    fn get_data_rid(&self) -> DbResult<crate::rid::RID> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.get_data_rid()
    }

    fn insert(&self, value: Constant, rid: crate::rid::RID) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.insert(value, rid)
    }

    fn delete(&self, value: Constant, rid: crate::rid::RID) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.delete(value, rid)
    }

    fn close(&self) -> DbResult<()> {
        let read = self.lock.read().map_err(DbError::lock)?;
        read.close()
    }
}
