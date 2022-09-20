use std::{cell::RefCell, ops::{Deref, DerefMut}, sync::PoisonError, thread_local};

thread_local! {
    pub static LOCK_LEVELS: RefCell<Vec<u32>> = RefCell::new(Vec::new());
}

/// Wrapper around a [`std::sync::Mutex`] which uses a thread local variable in order to check for
/// lock hierachy violations.
///
/// Each Mutex is assigned a level. Mutecies with higher levels must be acquired before mutices with
/// lower levels.
pub struct Mutex<T> {
    level: u32,
    inner: std::sync::Mutex<T>,
}

impl<T> Mutex<T> {
    /// Creates Mutex on level 0
    pub fn new(t: T) -> Self {
        Mutex {
            level: 0,
            inner: std::sync::Mutex::new(t),
        }
    }

    pub fn with_level(t: T, level: u32) -> Self {
        Mutex {
            level,
            inner: std::sync::Mutex::new(t),
        }
    }

    pub fn lock(&self) -> Result<MutexGuard<T>, PoisonError<std::sync::MutexGuard<'_, T>>> {
        LOCK_LEVELS.with(|levels| {
            let mut levels = levels.borrow_mut();
            if let Some(&lowest) = levels.last() {
                assert!(lowest > self.level)
            }
            levels.push(self.level);
        });
        self.inner.lock().map(|guard| MutexGuard {
            level: self.level,
            inner: guard,
        })
    }
}

pub struct MutexGuard<'a, T> {
    level: u32,
    inner: std::sync::MutexGuard<'a, T>,
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
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
    #[should_panic]
    fn should_panic_if_two_mutices_with_level_0_are_acquired() {
        let mutex_a = Mutex::new(5); // Level 0
        let mutex_b = Mutex::new(42); // also level 0
                                      // Fine, first mutex in thread
        let _guard_a = mutex_a.lock().unwrap();
        // Must panic, lock hierarchy violation
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
