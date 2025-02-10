#[cfg(debug_assertions)]
use std::{cell::RefCell, thread_local};

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
    ///   `RefCell` to acquire interior mutability.
    static LOCK_LEVELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug)]
pub(crate) struct Level {
    /// Level of this mutex in the hierarchy. Higher levels must be acquired first if locks are to
    /// be held simultaneously.
    #[cfg(debug_assertions)]
    pub(crate) level: u32,
}

impl Default for Level {
    #[inline]
    fn default() -> Self {
        Self::new(0)
    }
}

impl Level {
    #[inline]
    pub fn new(level: u32) -> Self {
        #[cfg(not(debug_assertions))]
        let _ = level;
        Self {
            #[cfg(debug_assertions)]
            level,
        }
    }

    #[inline]
    pub fn lock(&self) -> LevelGuard {
        #[cfg(debug_assertions)]
        LOCK_LEVELS.with(|levels| {
            let mut levels = levels.borrow_mut();
            if let Some(&lowest) = levels.last() {
                if lowest <= self.level {
                    panic!(
                        "Tried to acquire lock with level {} while a lock with level {} \
                        is acquired. This is a violation of lock hierarchies which \
                        could lead to deadlocks.",
                        self.level, lowest
                    )
                }
            }
            levels.push(self.level);
        });
        LevelGuard {
            #[cfg(debug_assertions)]
            level: self.level,
        }
    }
}

pub struct LevelGuard {
    #[cfg(debug_assertions)]
    pub(crate) level: u32,
}

#[cfg(debug_assertions)]
impl Drop for LevelGuard {
    #[inline]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(
        expected = "Tried to acquire lock with level 0 while a lock with level 0 is acquired. This is a violation of lock hierarchies which could lead to deadlocks."
    )]
    fn self_deadlock_detected() {
        let mutex = Level::new(0);
        let _guard_a = mutex.lock();
        // This must panic
        let _guard_b = mutex.lock();
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(
        expected = "Tried to acquire lock with level 0 while a lock with level 0 is acquired. This is a violation of lock hierarchies which could lead to deadlocks."
    )]
    fn panic_if_two_mutexes_with_level_0_are_acquired() {
        let mutex_a = Level::new(0);
        let mutex_b = Level::new(0);

        // Fine, first mutex in thread
        let _guard_a = mutex_a.lock();
        // Must panic, lock hierarchy violation
        let _guard_b = mutex_b.lock();
    }

    #[test]
    #[cfg(debug_assertions)]
    fn created_by_default_impl_should_be_level_0() {
        // This test would fail if mutex_a had any level greater than 0.
        let mutex = Level::default();
        assert_eq!(mutex.level, 0);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(
        expected = "Tried to acquire lock with level 1 while a lock with level 0 is acquired. This is a violation of lock hierarchies which could lead to deadlocks."
    )]
    fn panic_if_0_is_acquired_before_1() {
        let mutex_a = Level::new(0); // Level 0
        let mutex_b = Level::new(1); // Level 1

        // Fine, first mutex in thread
        let _guard_a = mutex_a.lock();
        // Must panic, lock hierarchy violation
        let _guard_b = mutex_b.lock();
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn should_not_check_in_release_build() {
        let mutex_a = Level::new(0);
        let mutex_b = Level::new(0);

        // Fine, first mutex in thread
        let _guard_a = mutex_a.lock();
        // Lock hierarchy violation, but we do not panic, since debug assertions are not active
        let _guard_b = mutex_b.lock();
    }

    #[test]
    fn two_level_0_in_succession() {
        let mutex_a = Level::new(5); // Level 0
        let mutex_b = Level::new(42); // also level 0
        {
            // Fine, first mutex in thread
            let _guard_a = mutex_a.lock();
        }
        // Fine, first mutex has already been dropped
        let _guard_b = mutex_b.lock();
    }

    #[test]
    fn simultaneous_lock_if_higher_is_acquired_first() {
        let mutex_a = Level::new(1);
        let mutex_b = Level::new(0);

        // Fine, first mutex in thread
        let _guard_a = mutex_a.lock();
        // Fine: 0 is lower level than 1
        let _guard_b = mutex_b.lock();
    }

    #[test]
    fn any_order_of_release() {
        let mutex_a = Level::new(2);
        let mutex_b = Level::new(1);
        let mutex_c = Level::new(0);

        // Fine, first mutex in thread
        let _guard_a = mutex_a.lock();
        // Fine: 0 is lower level than 1
        let guard_b = mutex_b.lock();
        let _guard_c = mutex_c.lock();
        #[allow(clippy::drop_non_drop)]
        drop(guard_b)
    }
}
