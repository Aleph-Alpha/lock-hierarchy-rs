use std::{
    ops::{Deref, DerefMut},
    sync::{LockResult, PoisonError},
};

use crate::level::{Level, LevelGuard};

/// Wrapper around a [`std::sync::RwLock`] which uses a thread local variable in order to check for
/// lock hierarchy violations in debug builds.
///
/// See the [crate level documentation](crate) for more general information.
///
/// ```
/// use lock_hierarchy::RwLock;
///
/// let mutex_a = RwLock::new(()); // Level 0
/// let mutex_b = RwLock::with_level((), 0); // also level 0
/// // Fine, first mutex in thread
/// let _guard_a = mutex_a.read().unwrap();
/// // Would panic, lock hierarchy violation
/// // let _guard_b = mutex_b.read().unwrap();
/// ```
#[derive(Debug, Default)]
pub struct RwLock<T> {
    inner: std::sync::RwLock<T>,
    level: Level,
}

impl<T> RwLock<T> {
    /// Creates a lock with level 0. Use this constructor if you want to get an error in debug builds
    /// every time you acquire another lock while holding this one.
    pub fn new(t: T) -> Self {
        Self::with_level(t, 0)
    }

    /// Creates a lock and assigns it a level in the lock hierarchy. Higher levels must be acquired
    /// first if locks are to be held simultaneously. This way we can ensure locks are always
    /// acquired in the same order. This prevents deadlocks.
    pub fn with_level(t: T, level: u32) -> Self {
        RwLock {
            inner: std::sync::RwLock::new(t),
            level: Level::new(level),
        }
    }

    /// See [std::sync::RwLock::read]
    pub fn read(
        &self,
    ) -> Result<RwLockReadGuard<T>, PoisonError<std::sync::RwLockReadGuard<'_, T>>> {
        self.inner.read().map(|guard| RwLockReadGuard {
            inner: guard,
            _level: self.level.lock(),
        })
    }

    /// See [std::sync::RwLock::write]
    pub fn write(
        &self,
    ) -> Result<RwLockWriteGuard<T>, PoisonError<std::sync::RwLockWriteGuard<'_, T>>> {
        self.inner.write().map(|guard| RwLockWriteGuard {
            inner: guard,
            _level: self.level.lock(),
        })
    }

    /// See [std::sync::RwLock::get_mut]
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        // No need to check hierarchy, this does not lock
        self.inner.get_mut()
    }

    /// See [std::sync::RwLock::into_inner]
    pub fn into_inner(self) -> LockResult<T> {
        // No need to check hierarchy, this does not lock
        self.inner.into_inner()
    }
}

impl<T> From<T> for RwLock<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    /// This is equivalent to [`RwLock::new`].
    fn from(value: T) -> Self {
        RwLock::new(value)
    }
}

pub struct RwLockReadGuard<'a, T> {
    inner: std::sync::RwLockReadGuard<'a, T>,
    _level: LevelGuard,
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.inner.deref()
    }
}

pub struct RwLockWriteGuard<'a, T> {
    inner: std::sync::RwLockWriteGuard<'a, T>,
    _level: LevelGuard,
}

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.inner.deref()
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}
