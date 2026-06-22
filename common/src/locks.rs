use std::{
    sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
    thread,
    time::{Duration, Instant},
};

use crate::{DbResult, error::DbError};

const TIMEOUT: Duration = if cfg!(test) {
    Duration::from_secs(1)
} else {
    Duration::from_secs(10)
};

pub struct TimedMutex<T>(Mutex<T>);

impl<T> TimedMutex<T> {
    pub fn new(data: T) -> TimedMutex<T> {
        TimedMutex(Mutex::new(data))
    }

    pub fn lock(&self) -> DbResult<MutexGuard<'_, T>> {
        let start = Instant::now();
        loop {
            match self.0.try_lock() {
                Ok(guard) => return Ok(guard),
                Err(_) if start.elapsed() >= TIMEOUT => return Err(DbError::LockTimeout),
                Err(_) => thread::sleep(Duration::from_millis(1)),
            }
        }
    }
}

pub struct TimedRwLock<T>(RwLock<T>);

impl<T> TimedRwLock<T> {
    pub fn new(data: T) -> TimedRwLock<T> {
        TimedRwLock(RwLock::new(data))
    }

    pub fn read(&self) -> DbResult<RwLockReadGuard<'_, T>> {
        let start = Instant::now();
        loop {
            match self.0.try_read() {
                Ok(guard) => return Ok(guard),
                Err(_) if start.elapsed() >= TIMEOUT => return Err(DbError::LockTimeout),
                Err(_) => thread::sleep(Duration::from_millis(1)),
            }
        }
    }

    pub fn write(&self) -> DbResult<RwLockWriteGuard<'_, T>> {
        let start = Instant::now();
        loop {
            match self.0.try_write() {
                Ok(guard) => return Ok(guard),
                Err(_) if start.elapsed() >= TIMEOUT => return Err(DbError::LockTimeout),
                Err(_) => thread::sleep(Duration::from_millis(1)),
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_lock_with_timeout() {
        let mutex = Mutex::new(0);
        let _lock1 = lock_with_timeout(&mutex, Duration::from_secs(1)).unwrap();
        let result = lock_with_timeout(&mutex, Duration::from_secs(2));
        assert!(matches!(result, Err(DbError::LockTimeout)));
    }

    #[test]
    fn timed_rw_lock_read() {
        let lock = TimedRwLock::new(0);
        let _r = lock.write();
        let result = lock.read();
        assert!(matches!(result, Err(DbError::LockTimeout)));
    }

    #[test]
    fn timed_rw_lock_write() {
        let lock = TimedRwLock::new(0);
        let _r = lock.read();
        let result = lock.write();
        assert!(matches!(result, Err(DbError::LockTimeout)));
    }

    #[test]
    fn timed_mutex() {
        let lock = TimedMutex::new(0);
        let _r = lock.lock();
        let result = lock.lock();
        assert!(matches!(result, Err(DbError::LockTimeout)));
    }
}
