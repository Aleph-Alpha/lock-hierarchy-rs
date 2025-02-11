use std::{
    fmt::{Debug, Display, Formatter},
    ops::{Deref, DerefMut},
    sync::LockResult,
};

use crate::{
    level::{Level, LevelGuard},
    map_guard,
};

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
    pub fn lock(&self) -> LockResult<MutexGuard<T>> {
        let level = self.level.lock();
        map_guard(self.inner.lock(), |guard| MutexGuard {
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

impl<T: Debug> Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl<T: Display> Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
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

#[cfg(test)]
mod tests {
    use std::{hint::black_box, sync::Arc, thread};

    use super::*;

    #[test]
    fn acquire_resource() {
        let mutex = Mutex::new(42);
        let guard = mutex.lock().unwrap();

        assert_eq!(42, *guard)
    }

    #[test]
    fn allow_mutation() {
        let mutex = Mutex::new(42);
        let mut guard = mutex.lock().unwrap();

        *guard = 43;

        assert_eq!(43, *guard)
    }

    #[test]
    fn multithreaded() {
        let mutex = Arc::new(Mutex::new(()));
        let thread = thread::spawn({
            let mutex = mutex.clone();
            move || {
                black_box(mutex.lock().unwrap());
            }
        });
        black_box(mutex.lock().unwrap());
        thread.join().unwrap();
    }

    #[test]
    #[should_panic(
        expected = "Tried to acquire lock with level 0 while a lock with level 0 is acquired. This is a violation of lock hierarchies which could lead to deadlocks."
    )]
    #[cfg(debug_assertions)]
    fn self_deadlock() {
        // This ensures that the level is locked in Mutex::lock before locking the std lock which might otherwise cause an unchecked deadlock
        let mutex = Mutex::new(());
        let _guard = mutex.lock().unwrap();
        let _guard = mutex.lock().unwrap();
    }

    #[test]
    #[should_panic(
        expected = "Tried to acquire lock with level 0 while a lock with level 0 is acquired. This is a violation of lock hierarchies which could lead to deadlocks."
    )]
    #[cfg(debug_assertions)]
    fn poisoned_lock() {
        let mutex = Mutex::new(());
        std::panic::catch_unwind(|| {
            let _guard = mutex.lock();
            panic!("lock is poisoned now");
        })
        .unwrap_err();

        let _guard_a = mutex.lock().unwrap_err().into_inner();
        let _guard_b = mutex.lock();
    }

    #[test]
    #[cfg(debug_assertions)]
    fn correct_level_locked() {
        let mutex = Mutex::with_level((), 1);
        let _guard_a = mutex.lock().unwrap();
        assert_eq!(_guard_a._level.level, 1);

        let mutex = Mutex::new(());
        let _guard_a = mutex.lock().unwrap();
        assert_eq!(_guard_a._level.level, 0);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn created_by_default_impl_should_be_level_0() {
        let mutex = Mutex::<()>::default();
        assert_eq!(mutex.level.level, 0);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn mutex_created_by_from_impl_should_be_level_0() {
        let mutex: Mutex<u8> = 42.into();
        assert_eq!(mutex.level.level, 0);
    }
}
