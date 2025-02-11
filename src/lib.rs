//! This crate offers debug assertions for violations of lock hierarchies. No runtime overhead or
//! protection occurs for release builds.
//!
//! Each lock is assigned a level. Locks with higher levels must be acquired before locks with
//! lower levels.
//! Both [RwLock] and [Mutex] use the same hierarchy.

mod level;
mod mutex;
mod rwlock;

use std::sync::{LockResult, PoisonError};

pub use mutex::{Mutex, MutexGuard};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub(crate) fn map_guard<G, F>(result: LockResult<G>, f: impl FnOnce(G) -> F) -> LockResult<F> {
    match result {
        Ok(guard) => Ok(f(guard)),
        Err(err) => Err(PoisonError::new(f(err.into_inner()))),
    }
}
