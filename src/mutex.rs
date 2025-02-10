use std::{
    ops::{Deref, DerefMut},
    sync::{LockResult, PoisonError},
};

use crate::level::{Level, LevelGuard};

/// Wrapper around a [`std::sync::Mutex`] which uses a thread local variable in order to check for
/// lock hierarchy violations in debug builds.
///
/// See the [crate level documentation](crate) for more general information.
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
    inner: std::sync::Mutex<T>,
    level: Level,
}

impl<T> Mutex<T> {
    /// Creates lock with level 0. Use this constructor if you want to get an error in debug builds
    /// every time you acquire another lock while holding this one.
    pub fn new(t: T) -> Self {
        Self::with_level(t, 0)
    }

    /// Creates a lock and assigns it a level in the lock hierarchy. Higher levels must be acquired
    /// first if locks are to be held simultaneously. This way we can ensure locks are always
    /// acquired in the same order. This prevents deadlocks.
    pub fn with_level(t: T, level: u32) -> Self {
        Mutex {
            inner: std::sync::Mutex::new(t),
            level: Level::new(level),
        }
    }

    /// See [std::sync::Mutex::lock]
    pub fn lock(&self) -> Result<MutexGuard<T>, PoisonError<std::sync::MutexGuard<'_, T>>> {
        let level = self.level.lock();
        self.inner.lock().map(|guard| MutexGuard {
            inner: guard,
            _level: level,
        })
    }

    /// See [std::sync::Mutex::get_mut]
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        // No need to check hierarchy, this does not lock
        self.inner.get_mut()
    }

    /// See [std::sync::Mutex::into_inner]
    pub fn into_inner(self) -> LockResult<T> {
        // No need to check hierarchy, this does not lock
        self.inner.into_inner()
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
