//! This crate offers debug assertions for violations of lock hierarchies. No runtime overhead or
//! protection occurs for release builds.

#[cfg(debug_assertions)]
use std::{cell::RefCell, thread_local};
use std::{
    ops::{Deref, DerefMut},
    sync::PoisonError,
};

#[cfg(debug_assertions)]
thread_local! {
    /// We hold a stack of thread local lock levels.
    ///
    /// * Thread local: We want to trace the lock level for each native system thread. Also making it
    ///   thread local implies that this needs no synchronization.
    /// * Stack: Just holding the current lock level would be insufficient in situations there locks
    ///   are released in a different order, from what they were acquired in. This way we can
    ///   support scenarios like e.g.: Acquire A, Acquire B, Release A, Acquire C, ...
    /// * RefCell: Static implies immutability in safe code, yet we want to mutate it. So we use a
    ///   `RefCell` to acquire interiour mutability.
    static LOCK_LEVELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

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
    /// Level of this mutex in the hierarchy. Higher levels must be acquired first if locks are to
    /// be held simultaniously.
    #[cfg(debug_assertions)]
    level: u32,
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
        // Explicitly ignore level in release builds
        #[cfg(not(debug_assertions))]
        let _ = level;
        Mutex {
            #[cfg(debug_assertions)]
            level,
            inner: std::sync::Mutex::new(t),
        }
    }

    pub fn lock(&self) -> Result<MutexGuard<T>, PoisonError<std::sync::MutexGuard<'_, T>>> {
        #[cfg(debug_assertions)]
        LOCK_LEVELS.with(|levels| {
            let mut levels = levels.borrow_mut();
            if let Some(&lowest) = levels.last() {
                if lowest <= self.level {
                    panic!(
                        "Tried to acquire lock to a mutex with level {}. Yet lock with level {} \
                        had been acquired first. This is a violation of lock hierarchies which \
                        could lead to deadlocks.",
                        self.level, lowest
                    )
                }
                assert!(lowest > self.level)
            }
            levels.push(self.level);
        });
        self.inner.lock().map(|guard| MutexGuard {
            #[cfg(debug_assertions)]
            level: self.level,
            inner: guard,
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
    #[cfg(debug_assertions)]
    level: u32,
    inner: std::sync::MutexGuard<'a, T>,
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        LOCK_LEVELS.with(|levels| {
            let mut levels = levels.borrow_mut();
            let index = levels
                .iter()
                .rposition(|&level| level == self.level)
                .expect("Position must exist, because we inserted it during lock!");
            levels.remove(index);
        });
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
