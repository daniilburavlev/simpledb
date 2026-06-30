use std::sync::Arc;

use common::{DbResult, error::DbError};
use transaction::transaction::Transaction;

use crate::buffer_needs::BufferNeeds;
use crate::element::Element;
use crate::value::Value;
use crate::{
    layout::Layout,
    scan::Scan,
    schema::{Schema, SchemaBuilder},
};

pub(crate) struct MultiBufferProductScan {
    tx: Arc<Transaction>,
    left: Box<Scan>,
    right: Option<Box<Scan>>,
    filename: String,
    layout: Layout,
    chunk_size: i32,
    next_block: i32,
    file_size: i32,
}

impl MultiBufferProductScan {
    pub(crate) fn new(
        tx: &Arc<Transaction>,
        left: Box<Scan>,
        filename: &str,
        layout: Layout,
    ) -> DbResult<Self> {
        let available = tx.available_buffs()? as i32;
        let file_size = tx.size(filename)? as i32;
        let mut scan = Self {
            tx: Arc::clone(tx),
            left,
            file_size: tx.size(filename)? as i32,
            filename: filename.to_string(),
            layout,
            chunk_size: BufferNeeds::best_factor(available, file_size),
            next_block: 0,
            right: None,
        };
        scan.before_first()?;
        Ok(scan)
    }


    pub(crate) fn use_next_chunk(&mut self) -> DbResult<bool> {
        if let Some(right) = self.right.take() {
            right.close()?;
        }
        if self.next_block >= self.file_size {
            return Ok(false);
        }
        let mut end = self.next_block + self.chunk_size - 1;
        if end >= self.file_size {
            end = self.file_size - 1;
        }
        let mut right = Box::new(Scan::chunk(
            &self.tx,
            &self.filename,
            self.layout.clone(),
            self.next_block,
            end,
        )?);
        self.left.before_first()?;
        self.left.next_row()?;
        right.before_first()?;
        self.right = Some(right);
        self.next_block = end + 1;
        Ok(true)
    }

    pub(crate) fn before_first(&mut self) -> DbResult<()> {
        self.next_block = 0;
        self.use_next_chunk()?;
        Ok(())
    }

    pub(crate) fn next(&mut self) -> DbResult<bool> {
        loop {
            if let Some(right) = self.right.as_mut() {
                if right.next_row()? {
                    return Ok(true);
                }
                right.before_first()?;
                if right.next_row()? && self.left.next_row()? {
                    return Ok(true);
                }
            }
            if !self.use_next_chunk()? {
                return Ok(false);
            }
        }
    }

    pub(crate) fn get_i32(&self, field_name: &Element) -> DbResult<i32> {
        if self.left.has_field(field_name)? {
            self.left.get_i32(field_name)
        } else {
            self.right_scan()?.get_i32(field_name)
        }
    }

    pub(crate) fn get_string(&self, field_name: &Element) -> DbResult<String> {
        if self.left.has_field(field_name)? {
            self.left.get_string(field_name)
        } else {
            self.right_scan()?.get_string(field_name)
        }
    }

    pub(crate) fn get_val(&self, field_name: &Element) -> DbResult<Value> {
        if self.left.has_field(field_name)? {
            self.left.get_val(field_name)
        } else {
            self.right_scan()?.get_val(field_name)
        }
    }

    pub(crate) fn has_field(&self, field_name: &Element) -> DbResult<bool> {
        if self.left.has_field(field_name)? {
            return Ok(true);
        }
        match &self.right {
            Some(right) => right.has_field(field_name),
            None => Ok(false),
        }
    }

    pub(crate) fn close(&self) -> DbResult<()> {
        self.left.close()?;
        if let Some(right) = &self.right {
            right.close()?;
        }
        Ok(())
    }

    pub(crate) fn schema(&self) -> DbResult<Schema> {
        let mut builder = SchemaBuilder::default().add_all(&self.left.schema()?);
        if let Some(right) = &self.right {
            builder = builder.add_all(&right.schema()?);
        }
        Ok(builder.build())
    }

    fn right_scan(&self) -> DbResult<&Scan> {
        self.right
            .as_deref()
            .ok_or_else(|| DbError::other("multibuffer right chunk not open"))
    }
}
