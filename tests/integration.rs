
use lock_hierarchy::Mutex;

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
