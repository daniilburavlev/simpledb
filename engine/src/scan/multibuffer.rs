use std::{cell::RefCell, rc::Rc, sync::Arc};

use common::{DbResult, error::DbError};
use transaction::transaction::Transaction;

use crate::{
    buffer_needs::BufferNeeds,
    layout::Layout,
    scan::{Scan, chunk::ChunkScan, product::ProductScan},
    schema::Schema,
};

struct MultiBufferProductScanInner {
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

impl MultiBufferProductScanInner {
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
        if let Some(right) = self.right.take() {
            right.close()?;
        }
        self.prod = None;
        if self.next_block >= self.file_size {
            return Ok(false);
        }
        let mut end = self.next_block + self.chunk_size - 1;
        if end >= self.file_size {
            end = self.file_size - 1;
        }
        let right: Rc<dyn Scan> = Rc::new(ChunkScan::new(
            &self.tx,
            &self.filename,
            &self.layout,
            self.next_block,
            end,
        )?);
        self.left.before_first()?;
        self.prod = Some(Rc::new(ProductScan::new(self.left.clone(), right.clone())?));
        self.right = Some(right);
        self.next_block = end + 1;
        Ok(true)
    }

    fn before_first(&mut self) -> DbResult<()> {
        self.next_block = 0;
        self.use_next_chunk()?;
        Ok(())
    }

    fn next(&mut self) -> DbResult<bool> {
        loop {
            let has_next = match &self.prod {
                Some(prod) => prod.next()?,
                None => false,
            };
            if has_next {
                return Ok(true);
            }
            if !self.use_next_chunk()? {
                return Ok(false);
            }
        }
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
        self.left.close()?;
        if let Some(right) = &self.right {
            right.close()?;
        }
        Ok(())
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
    lock: RefCell<MultiBufferProductScanInner>,
}

impl MultiBufferProductScan {
    pub(crate) fn new(
        tx: &Arc<Transaction>,
        left: &Rc<dyn Scan>,
        filename: &str,
        layout: &Arc<Layout>,
    ) -> DbResult<Self> {
        Ok(Self {
            lock: RefCell::new(MultiBufferProductScanInner::new(
                tx, left, filename, layout,
            )?),
        })
    }
}

impl Scan for MultiBufferProductScan {
    fn before_first(&self) -> DbResult<()> {
        let mut write = self.lock.borrow_mut();
        write.before_first()
    }

    fn next(&self) -> DbResult<bool> {
        let mut write = self.lock.borrow_mut();
        write.next()
    }

    fn get_i32(&self, field_name: &str) -> DbResult<i32> {
        let read = self.lock.borrow();
        read.get_i32(field_name)
    }

    fn get_string(&self, field_name: &str) -> DbResult<String> {
        let read = self.lock.borrow();
        read.get_string(field_name)
    }

    fn get_val(&self, field_name: &str) -> DbResult<crate::constant::Constant> {
        let read = self.lock.borrow();
        read.get_val(field_name)
    }

    fn has_field(&self, field_name: &str) -> DbResult<bool> {
        let read = self.lock.borrow();
        read.has_field(field_name)
    }

    fn close(&self) -> DbResult<()> {
        let read = self.lock.borrow();
        read.close()
    }

    fn schema(&self) -> DbResult<Arc<Schema>> {
        let read = self.lock.borrow();
        read.schema()
    }
}
