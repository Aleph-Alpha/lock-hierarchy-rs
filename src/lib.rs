//! This crate offers debug assertions for violations of lock hierarchies. No runtime overhead or
//! protection occurs for release builds.
//!
//! Each lock is assigned a level. Locks with higher levels must be acquired before locks with
//! lower levels.
//! Both [RwLock] and [Mutex] use the same hierarchy.

mod level;
mod mutex;
mod rwlock;

pub use mutex::{Mutex, MutexGuard};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
