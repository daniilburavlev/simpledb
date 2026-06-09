use buffer::{buffer::Buffer, mgr::BufferMgr};
use common::DbResult;
use common::error::DbError;
use file::block::BlockId;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
struct Pins(HashMap<BlockId, u16>);

impl Pins {
    fn contains(&self, block: &BlockId) -> bool {
        let Some(count) = self.0.get(block) else {
            return false;
        };
        *count > 0
    }

    fn add(&mut self, block: &BlockId) {
        self.0
            .entry(block.clone())
            .and_modify(|v| *v += 1)
            .or_insert(1);
    }

    fn remove(&mut self, block: &BlockId) {
        self.0
            .entry(block.clone())
            .and_modify(|v| {
                if *v > 0 {
                    *v -= 1
                }
            })
            .or_insert(0);
    }

    fn clear(&mut self) {
        self.0.clear();
    }
}

struct BufferListLock {
    buffers: HashMap<BlockId, Buffer>,
    pins: Pins,
    bm: Arc<BufferMgr>,
}

impl BufferListLock {
    pub fn new(bm: &Arc<BufferMgr>) -> Self {
        Self {
            buffers: HashMap::new(),
            pins: Pins::default(),
            bm: Arc::clone(bm),
        }
    }

    pub fn get_buffer(&self, block: &BlockId) -> Option<Buffer> {
        self.buffers.get(block).cloned()
    }

    pub fn pin(&mut self, block: &BlockId) -> DbResult<()> {
        let buffer = self.bm.pin(block)?;
        self.buffers.insert(block.clone(), buffer);
        self.pins.add(block);
        Ok(())
    }

    pub fn unpin(&mut self, block: &BlockId) -> DbResult<()> {
        if let Some(buffer) = self.buffers.get(block) {
            self.bm.unpin(buffer.clone())?;
            self.pins.remove(block);
            if !self.pins.contains(block) {
                self.buffers.remove(block);
            }
        }
        Ok(())
    }

    pub fn unpin_all(&mut self) -> DbResult<()> {
        for (block, _) in self.pins.0.iter() {
            if let Some(buffer) = self.buffers.get(block).cloned() {
                self.bm.unpin(buffer)?;
            }
        }
        self.buffers.clear();
        self.pins.clear();
        Ok(())
    }
}

pub struct BufferList {
    lock: RwLock<BufferListLock>,
}

impl BufferList {
    pub fn new(bm: &Arc<BufferMgr>) -> Self {
        Self {
            lock: RwLock::new(BufferListLock::new(bm)),
        }
    }

    pub fn get_buffer(&self, block: &BlockId) -> DbResult<Option<Buffer>> {
        let read = self.lock.read().map_err(DbError::lock)?;
        Ok(read.get_buffer(block))
    }

    pub fn pin(&self, block: &BlockId) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.pin(block)
    }

    pub fn unpin(&self, block: &BlockId) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.unpin(block)
    }

    pub fn unpin_all(&self) -> DbResult<()> {
        let mut write = self.lock.write().map_err(DbError::lock)?;
        write.unpin_all()
    }
}
