//! This crate offers debug assertions for violations of lock hierarchies. No runtime overhead or
//! protection occurs for release builds.

mod level;

use std::sync::LockResult;

use crate::level::{Level, LevelGuard};
use std::{
    ops::{Deref, DerefMut},
    sync::PoisonError,
};

/// Wrapper around a [`std::sync::Mutex`] which uses a thread local variable in order to check for
/// lock hierachy violations in debug builds.
///
/// Each Mutex is assigned a level. Mutexes with higher levels must be acquired before mutexes  with
/// lower levels.
///
/// ```
/// use lock_hierarchy::Mutex;
///
/// let mutex_a = Mutex::new(()); // Level 0
/// let mutex_b = Mutex::with_level((), 0); // also level 0
/// // Fine, first mutex in thread
/// let _guard_a = mutex_a.lock().unwrap();
/// // Would panic, lock hierarchy violation
/// // let _guard_b = mutex_b.lock().unwrap();
/// ```
#[derive(Debug, Default)]
pub struct Mutex<T> {
    level: Level,
    inner: std::sync::Mutex<T>,
}

impl<T> Mutex<T> {
    /// Creates Mutex with level 0. Use this constructor if you want to get an error in debug builds
    /// every time you acquire another mutex while holding this one.
    pub fn new(t: T) -> Self {
        Self::with_level(t, 0)
    }

    /// Creates a mutex and assigns it a level in the lock hierarchy. Higher levels must be acquired
    /// first if locks are to be held simultaniously. This way we can ensure locks are always
    /// acquired in the same order. This prevents deadlocks.
    pub fn with_level(t: T, level: u32) -> Self {
        Mutex {
            level: Level::new(level),
            inner: std::sync::Mutex::new(t),
        }
    }

    /// See [std::sync::Mutex::lock]
    pub fn lock(&self) -> Result<MutexGuard<T>, PoisonError<std::sync::MutexGuard<'_, T>>> {
        self.inner.lock().map(|guard| MutexGuard {
            inner: guard,
            _level: self.level.lock(),
        })
    }
}

impl<T> From<T> for Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    /// This is equivalent to [`Mutex::new`].
    fn from(value: T) -> Self {
        Mutex::new(value)
    }
}

pub struct MutexGuard<'a, T> {
    inner: std::sync::MutexGuard<'a, T>,
    _level: LevelGuard,
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.inner.deref()
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}
