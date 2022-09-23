//! This crate offers debug assertions for violations of lock hierarchies. No runtime overhead or
//! protection occurs for release builds.

#[cfg(debug_assertions)]
use std::{cell::RefCell, thread_local};
use std::{ops::{Deref, DerefMut}, sync::PoisonError};

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
    pub static LOCK_LEVELS: RefCell<Vec<u32>> = RefCell::new(Vec::new());
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
/// // Must panic, lock hierarchy violation
/// let _guard_b = mutex_b.lock().unwrap();
/// ```
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

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.inner.deref()
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {

    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_resource() {
        let mutex = Mutex::new(42);
        let guard = mutex.lock().unwrap();

        assert_eq!(42, *guard)
    }

    #[test]
    fn should_allow_mutation() {
        let mutex = Mutex::new(42);
        let mut guard = mutex.lock().unwrap();

        *guard = 43;

        assert_eq!(43, *guard)
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn should_panic_if_two_mutices_with_level_0_are_acquired() {
        let mutex_a = Mutex::new(()); // Level 0
        let mutex_b = Mutex::new(()); // also level 0
        // Fine, first mutex in thread
        let _guard_a = mutex_a.lock().unwrap();
        // Must panic, lock hierarchy violation
        let _guard_b = mutex_b.lock().unwrap();
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn should_not_check_in_release_build() {
        let mutex_a = Mutex::new(5); // Level 0
        let mutex_b = Mutex::new(42); // also level 0
                                      // Fine, first mutex in thread
        let _guard_a = mutex_a.lock().unwrap();
        // Lock hierarchy violation, but we do not panic, since debug assertions are not active
        let _guard_b = mutex_b.lock().unwrap();
    }

    #[test]
    fn should_allow_for_two_level_0_in_succession() {
        let mutex_a = Mutex::new(5); // Level 0
        let mutex_b = Mutex::new(42); // also level 0
                                      // Fine, first mutex in thread
        let guard_a = mutex_a.lock().unwrap();
        drop(guard_a);
        // Fine, first mutext has already been dropped
        let _guard_b = mutex_b.lock().unwrap();
    }

    #[test]
    fn should_allow_for_simultanous_lock_if_higher_is_acquired_first() {
        let mutex_a = Mutex::with_level(5, 1); // Level 1
        let mutex_b = Mutex::new(42); // also level 0
        // Fine, first mutex in thread
        let _guard_a = mutex_a.lock().unwrap();
        // Fine: 0 is lower level than 1
        let _guard_b = mutex_b.lock().unwrap();
    }

    #[test]
    fn should_allow_for_any_order_of_release() {
        let mutex_a = Mutex::with_level((), 2);
        let mutex_b = Mutex::with_level((), 1);
        let mutex_c = Mutex::new(());
        // Fine, first mutex in thread
        let _guard_a = mutex_a.lock().unwrap();
        // Fine: 0 is lower level than 1
        let guard_b = mutex_b.lock().unwrap();
        let _guard_c = mutex_c.lock().unwrap();
        drop(guard_b)
    }
}
