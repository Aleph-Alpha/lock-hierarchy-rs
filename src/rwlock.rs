use std::{
    fmt::{Debug, Display, Formatter},
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
        let level = self.level.lock();
        self.inner.read().map(|guard| RwLockReadGuard {
            inner: guard,
            _level: level,
        })
    }

    /// See [std::sync::RwLock::write]
    pub fn write(
        &self,
    ) -> Result<RwLockWriteGuard<T>, PoisonError<std::sync::RwLockWriteGuard<'_, T>>> {
        let level = self.level.lock();
        self.inner.write().map(|guard| RwLockWriteGuard {
            inner: guard,
            _level: level,
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

impl<'a, T: Debug> Debug for RwLockReadGuard<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl<'a, T: Display> Display for RwLockReadGuard<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
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

impl<'a, T: Debug> Debug for RwLockWriteGuard<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl<'a, T: Display> Display for RwLockWriteGuard<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_resource() {
        let mutex = RwLock::new(42);
        let guard = mutex.read().unwrap();
        assert_eq!(42, *guard);
        drop(guard);

        let guard = mutex.write().unwrap();
        assert_eq!(42, *guard);
        drop(guard);
    }

    #[test]
    fn allow_mutation() {
        let mutex = RwLock::new(42);
        let mut guard = mutex.write().unwrap();

        *guard = 43;

        assert_eq!(43, *guard)
    }

    #[test]
    #[should_panic(
        expected = "Tried to acquire lock with level 0 while a lock with level 0 is acquired. This is a violation of lock hierarchies which could lead to deadlocks."
    )]
    #[cfg(debug_assertions)]
    fn self_deadlock_write() {
        // This ensures that the level is locked in RwLock::write before locking the std lock which might otherwise cause a deadlock
        let mutex = RwLock::new(());
        let _guard = mutex.read().unwrap();
        let _guard = mutex.write().unwrap();
    }

    #[test]
    #[should_panic(
        expected = "Tried to acquire lock with level 0 while a lock with level 0 is acquired. This is a violation of lock hierarchies which could lead to deadlocks."
    )]
    #[cfg(debug_assertions)]
    fn self_deadlock_read() {
        // This ensures that the level is locked in RwLock::read before locking the std lock which might otherwise cause an unchecked deadlock
        let mutex = RwLock::new(());
        let _guard = mutex.read().unwrap();
        let _guard = mutex.read().unwrap();
    }

    #[test]
    #[cfg(debug_assertions)]
    fn correct_level_locked() {
        let mutex = RwLock::with_level((), 1);
        let guard = mutex.read().unwrap();
        assert_eq!(guard._level.level, 1);
        drop(guard);
        let guard = mutex.write().unwrap();
        assert_eq!(guard._level.level, 1);
        drop(guard);

        let mutex = RwLock::new(());
        let guard = mutex.read().unwrap();
        assert_eq!(guard._level.level, 0);
        drop(guard);
        let guard = mutex.write().unwrap();
        assert_eq!(guard._level.level, 0);
        drop(guard);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn created_by_default_impl_should_be_level_0() {
        let mutex = RwLock::<()>::default();
        assert_eq!(mutex.level.level, 0);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn mutex_created_by_from_impl_should_be_level_0() {
        let mutex: RwLock<u8> = 42.into();
        assert_eq!(mutex.level.level, 0);
    }
}
