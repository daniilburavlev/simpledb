use std::{rc::Rc, sync::Arc};

use common::{DbResult, error::DbError, locks::TimedRwLock};
use transaction::transaction::Transaction;

use crate::{
    buffer_needs::BufferNeeds,
    layout::Layout,
    scan::{Scan, chunk::ChunkScan, product::ProductScan},
    schema::Schema,
};

pub(crate) struct MultiBufferProductScanLock {
    tx: Arc<Transaction>,
    left: Rc<dyn Scan>,
    right: Option<Rc<dyn Scan>>,
    prod: Option<Rc<dyn Scan>>,
    filename: String,
    layout: Arc<Layout>,
    chunk_size: i32,
    next_block: i32,
    file_size: i32,
}

impl MultiBufferProductScanLock {
    pub(crate) fn new(
        tx: &Arc<Transaction>,
        left: &Rc<dyn Scan>,
        filename: &str,
        layout: &Arc<Layout>,
    ) -> DbResult<Self> {
        let available = tx.available_buffs()? as i32;
        let file_size = tx.size(filename)? as i32;
        let mut scan = Self {
            tx: Arc::clone(tx),
            left: Rc::clone(left),
            file_size: tx.size(filename)? as i32,
            filename: filename.to_string(),
            layout: Arc::clone(layout),
            chunk_size: BufferNeeds::best_factor(available, file_size),
            next_block: 0,
            prod: None,
            right: None,
        };
        scan.before_first()?;
        Ok(scan)
    }

    fn use_next_chunk(&mut self) -> DbResult<bool> {
        if let Some(right) = &self.right {
            right.close()?;
        }
        if self.next_block >= self.file_size {
            return Ok(false);
        }
        let mut end = self.next_block + self.chunk_size - 1;
        if end >= self.file_size {
            end = self.file_size - 1;
        }
        self.right = Some(Rc::new(ChunkScan::new(
            &self.tx,
            &self.filename,
            &self.layout,
            self.next_block,
            end,
        )?));
        self.left.before_first()?;
        if let Some(right) = &self.right {
            self.prod = Some(Rc::new(ProductScan::new(self.left.clone(), right.clone())?));
        } else {
            panic!("right scan is empty");
        }
        Ok(true)
    }

    fn before_first(&mut self) -> DbResult<()> {
        self.next_block = 0;
        self.use_next_chunk()?;
        Ok(())
    }

    fn next(&mut self) -> DbResult<bool> {
        while let Some(prod) = &self.prod
            && prod.next()?
        {
            if !self.use_next_chunk()? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        if let Some(prod) = &self.prod {
            prod.get_i32(field_name)
        } else {
            Err(DbError::other("cannot get prod"))
        }
    }

    fn get_string(&self, field_name: &str) -> DbResult<String> {
        if let Some(prod) = &self.prod {
            prod.get_string(field_name)
        } else {
            Err(DbError::other("cannot get prod"))
        }
    }

    fn get_val(&self, field_name: &str) -> DbResult<crate::constant::Constant> {
        if let Some(prod) = &self.prod {
            prod.get_val(field_name)
        } else {
            Err(DbError::other("cannot get prod"))
        }
    }

    fn has_field(&self, field_name: &str) -> DbResult<bool> {
        if let Some(prod) = &self.prod {
            prod.has_field(field_name)
        } else {
            Err(DbError::other("cannot get prod"))
        }
    }

    fn close(&self) -> DbResult<()> {
        if let Some(prod) = &self.prod {
            prod.close()
        } else {
            Err(DbError::other("cannot get prod"))
        }
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        if let Some(prod) = &self.prod {
            prod.schema()
        } else {
            Err(DbError::other("cannot get prod"))
        }
    }
}

pub(crate) struct MultiBufferProductScan {
    lock: TimedRwLock<MultiBufferProductScanLock>,
}

impl MultiBufferProductScan {
    pub(crate) fn new(
        tx: &Arc<Transaction>,
        left: &Rc<dyn Scan>,
        filename: &str,
        layout: &Arc<Layout>,
    ) -> DbResult<Self> {
        Ok(Self {
            lock: TimedRwLock::new(MultiBufferProductScanLock::new(tx, left, filename, layout)?),
        })
    }
}

impl Scan for MultiBufferProductScan {
    fn before_first(&self) -> DbResult<()> {
        let mut write = self.lock.write()?;
        write.before_first()
    }

    fn next(&self) -> DbResult<bool> {
        let mut write = self.lock.write()?;
        write.next()
    }

    fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        let read = self.lock.read()?;
        read.get_i32(field_name)
    }

    fn get_string(&self, field_name: &str) -> DbResult<String> {
        let read = self.lock.read()?;
        read.get_string(field_name)
    }

    fn get_val(&self, field_name: &str) -> DbResult<crate::constant::Constant> {
        let read = self.lock.read()?;
        read.get_val(field_name)
    }

    fn has_field(&self, field_name: &str) -> DbResult<bool> {
        let read = self.lock.read()?;
        read.has_field(field_name)
    }

    fn close(&self) -> DbResult<()> {
        let read = self.lock.read()?;
        read.close()
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        let read = self.lock.read()?;
        read.schema()
    }
}
