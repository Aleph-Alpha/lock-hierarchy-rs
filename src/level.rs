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
pub struct Level {
    /// Level of this mutex in the hierarchy. Higher levels must be acquired first if locks are to
    /// be held simultaneously.
    #[cfg(debug_assertions)]
    level: u32,
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
                        "Tried to acquire lock to a mutex with level {}. Yet lock with level {} \
                        had been acquired first. This is a violation of lock hierarchies which \
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
    level: u32,
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
