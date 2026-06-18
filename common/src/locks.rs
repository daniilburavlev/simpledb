use std::{
    sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
    thread,
    time::{Duration, Instant},
};

use crate::{DbResult, error::DbError};

const TIMEOUT: Duration = Duration::from_secs(10);

pub struct TimedMutex<T>(Mutex<T>);

impl<T> TimedMutex<T> {
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
