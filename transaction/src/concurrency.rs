pub mod locks;
pub mod mgr;

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        sync::{
            Arc,
            mpsc::{self, Sender},
        },
        thread,
        time::Duration,
    };

    use buffer::mgr::BufferMgr;
    use file::{block::BlockId, mgr::FileMgr};
    use log::mgr::LogMgr;
    use tempfile::tempdir;

    use crate::{lock_table::LockTable, transaction::Transaction, txnum_generator::TxNumGenerator};

    #[derive(Hash, PartialEq, Eq)]
    enum TxResult {
        AS1,
        AS2,
        BS1,
        BX2,
        CS2,
        CX1,
    }

    #[test]
    fn concurrency() {
        let dir = tempdir().unwrap();
        let fm = Arc::new(FileMgr::new(dir.path(), 512).unwrap());
        let lm = Arc::new(LogMgr::new(&fm, "testlog".to_string()).unwrap());
        let bm = Arc::new(BufferMgr::new(&fm, &lm, 8).unwrap());
        let txnum_generator = TxNumGenerator::default();
        let lock_table = Arc::new(LockTable::default());

        let (tx_tx, tx_rx) = mpsc::channel();

        let tx_a = Transaction::new(&txnum_generator, &fm, &lm, &bm, &lock_table).unwrap();
        let tx_tx_a = tx_tx.clone();
        thread::spawn(move || a(tx_a, tx_tx_a));
        thread::sleep(Duration::from_millis(100));

        let tx_tx_b = tx_tx.clone();
        let tx_b = Transaction::new(&txnum_generator, &fm, &lm, &bm, &lock_table).unwrap();
        thread::spawn(move || b(tx_b, tx_tx_b));
        thread::sleep(Duration::from_millis(100));

        let tx_tx_c = tx_tx.clone();
        let tx_c = Transaction::new(&txnum_generator, &fm, &lm, &bm, &lock_table).unwrap();
        thread::spawn(move || c(tx_c, tx_tx_c));
        thread::sleep(Duration::from_millis(100));

        let mut expected = HashSet::new();
        expected.insert(TxResult::AS1);
        expected.insert(TxResult::AS2);
        expected.insert(TxResult::BS1);
        expected.insert(TxResult::BS1);
        expected.insert(TxResult::CS2);
        expected.insert(TxResult::CX1);

        loop {
            let Ok(tx) = tx_rx.recv_timeout(Duration::from_secs(5)) else {
                panic!("channel closed");
            };
            expected.remove(&tx);
            if expected.is_empty() {
                break;
            }
        }
    }

    fn a(tx: Transaction, sender: Sender<TxResult>) {
        let block1 = BlockId::new("testfile", 1);
        let block2 = BlockId::new("testfile", 2);
        tx.pin(&block1).unwrap();
        tx.pin(&block2).unwrap();
        println!("tx A: request slock 1");
        tx.get_i32(&block1, 0).unwrap();
        sender.send(TxResult::AS1).unwrap();
        println!("tx A: received slock 1");
        thread::sleep(Duration::from_secs(1));
        println!("tx A: request slock 2");
        tx.get_i32(&block2, 0).unwrap();
        sender.send(TxResult::AS2).unwrap();
        println!("tx A: received slock 2");
        tx.commit().unwrap();
    }

    fn b(tx: Transaction, sender: Sender<TxResult>) {
        let block1 = BlockId::new("testfile", 1);
        let block2 = BlockId::new("testfile", 2);
        tx.pin(&block1).unwrap();
        tx.pin(&block2).unwrap();
        println!("tx B: request xlock 2");
        tx.set_i32(&block2, 0, 0, false).unwrap();
        sender.send(TxResult::BX2).unwrap();
        println!("tx B: received xlock 2");
        thread::sleep(Duration::from_secs(1));
        println!("tx B: request slock 1");
        tx.get_i32(&block1, 0).unwrap();
        sender.send(TxResult::BS1).unwrap();
        println!("tx B: received slock 1");
        tx.commit().unwrap();
    }

    fn c(tx: Transaction, sender: Sender<TxResult>) {
        let block1 = BlockId::new("testfile", 1);
        let block2 = BlockId::new("testfile", 2);
        tx.pin(&block1).unwrap();
        tx.pin(&block2).unwrap();
        println!("tx C: request xlock 1");
        tx.set_i32(&block1, 0, 213, false).unwrap();
        sender.send(TxResult::CX1).unwrap();
        println!("tx C: received xlock 1");
        thread::sleep(Duration::from_secs(1));
        println!("tx C: request slock 2");
        tx.get_i32(&block2, 0).unwrap();
        sender.send(TxResult::CS2).unwrap();
        println!("tx C: received slock 2");
        tx.commit().unwrap();
    }
}
