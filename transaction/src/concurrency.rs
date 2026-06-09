pub mod locks;
pub mod mgr;

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread, time::Duration};

    use buffer::mgr::BufferMgr;
    use file::{block::BlockId, mgr::FileMgr};
    use log::mgr::LogMgr;
    use tempfile::tempdir;

    use crate::{lock_table::LockTable, transaction::Transaction, txnum_generator::TxNumGenerator};

    #[test]
    fn concurrency() {
        let dir = tempdir().unwrap();
        let fm = Arc::new(FileMgr::new(dir.path(), 512).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "testlog".to_string()).unwrap());
        let bm = Arc::new(BufferMgr::new(&fm, &lm, 8).unwrap());
        let txnum_generator = TxNumGenerator::default();
        let lock_table = Arc::new(LockTable::default());
        let tx_a = Transaction::new(&txnum_generator, &fm, &lm, &bm, &lock_table).unwrap();
        let a = thread::spawn(move || a(tx_a));
        thread::sleep(Duration::from_millis(100));
        let tx_b = Transaction::new(&txnum_generator, &fm, &lm, &bm, &lock_table).unwrap();
        let b = thread::spawn(move || b(tx_b));
        thread::sleep(Duration::from_millis(100));
        let tx_c = Transaction::new(&txnum_generator, &fm, &lm, &bm, &lock_table).unwrap();
        let c = thread::spawn(move || c(tx_c));
        thread::sleep(Duration::from_millis(100));
        a.join().unwrap();
        b.join().unwrap();
        c.join().unwrap();
    }

    fn a(tx: Transaction) {
        let block1 = BlockId::new("testfile", 1);
        let block2 = BlockId::new("testfile", 2);
        tx.pin(&block1).unwrap();
        tx.pin(&block2).unwrap();
        println!("tx A: request slock 1");
        tx.get_i32(&block1, 0).unwrap();
        println!("tx A: received slock 1");
        thread::sleep(Duration::from_secs(1));
        println!("tx A: request slock 2");
        tx.get_i32(&block2, 0).unwrap();
        println!("tx A: received slock 2");
        tx.commit().unwrap();
    }

    fn b(tx: Transaction) {
        let block1 = BlockId::new("testfile", 1);
        let block2 = BlockId::new("testfile", 2);
        tx.pin(&block1).unwrap();
        tx.pin(&block2).unwrap();
        println!("tx B: request xlock 2");
        tx.set_i32(&block2, 0, 0, false).unwrap();
        println!("tx B: received xlock 2");
        thread::sleep(Duration::from_secs(1));
        println!("tx B: request slock 1");
        tx.get_i32(&block1, 0).unwrap();
        println!("tx B: received slock 1");
        tx.commit().unwrap();
    }

    fn c(tx: Transaction) {
        let block1 = BlockId::new("testfile", 1);
        let block2 = BlockId::new("testfile", 2);
        tx.pin(&block1).unwrap();
        tx.pin(&block2).unwrap();
        println!("tx C: request xlock 1");
        tx.get_i32(&block1, 0).unwrap();
        println!("tx C: received xlock 1");
        thread::sleep(Duration::from_secs(1));
        println!("tx C: request slock 2");
        tx.get_i32(&block2, 0).unwrap();
        println!("tx C: received slock 2");
        tx.commit().unwrap();
    }
}
