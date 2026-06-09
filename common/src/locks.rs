use std::{
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use crate::{DbResult, error::DbError};

pub fn lock_with_timeout<T>(mutex: &Mutex<T>, timeout: Duration) -> DbResult<MutexGuard<'_, T>> {
    let start = Instant::now();
    loop {
        match mutex.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(_) if start.elapsed() >= timeout => return Err(DbError::LockTimeout),
            Err(_) => thread::sleep(Duration::from_millis(1)),
        }
    }
}
