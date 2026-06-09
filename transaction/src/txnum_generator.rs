use std::sync::atomic::AtomicI32;

pub struct TxNumGenerator(AtomicI32);

impl TxNumGenerator {
    pub(crate) fn next(&self) -> i32 {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for TxNumGenerator {
    fn default() -> Self {
        Self(AtomicI32::new(0))
    }
}
