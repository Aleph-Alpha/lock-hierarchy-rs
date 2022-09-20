# Lock hierarchy

This Rust crate offers debug assertions for violations of lock hierarchies. No runtime overhead or protection occurs for release builds.

## Usage

```rust
use lock_hierarchy::Mutex;

let mutex_a = Mutex::new(()); // Level 0
let mutex_b = Mutex::with_level((), 0); // also level 0
// Fine, first mutex in thread
let _guard_a = mutex_a.lock().unwrap();
// Must panic, lock hierarchy violation
let _guard_b = mutex_b.lock().unwrap();
```

```rust
use lock_hierarchy::Mutex;

let mutex_a = Mutex::with_level((), 1); // Level 1
let mutex_b = Mutex::new(()); // level 0
// Fine, first mutex in thread
let _guard_a = mutex_a.lock().unwrap();
// Fine: 0 is lower level than 1
let _guard_b = mutex_b.lock().unwrap();
```