//! Test for issue #537 - Rust destructors should be called when PHP bailout occurs.
//!
//! This test verifies that when PHP triggers a bailout (e.g., via `exit()`), Rust
//! destructors are properly called before the bailout is re-triggered.
//!
//! There are two mechanisms for ensuring cleanup:
//!
//! 1. **Using `try_call`**: When calling PHP code via `try_call`, bailouts are caught
//!    internally and the function returns normally, allowing regular Rust destructors to run.
//!
//! 2. **Using `BailoutGuard`**: For values that MUST be cleaned up even if bailout occurs
//!    directly (not via `try_call`), wrap them in `BailoutGuard`. This heap-allocates the
//!    value and registers a cleanup callback that runs when bailout is caught.

use ext_php_rs::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// Static counter to track how many times the destructor was called.
/// This is used to verify destructors run even when bailout occurs.
static DROP_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A struct that increments a counter when dropped.
/// Used to verify destructors are called during bailout.
struct DropTracker {
    _id: u32,
}

impl DropTracker {
    fn new(id: u32) -> Self {
        Self { _id: id }
    }
}

impl Drop for DropTracker {
    fn drop(&mut self) {
        // Increment the counter to prove the destructor was called
        DROP_COUNTER.fetch_add(1, Ordering::SeqCst);
    }
}

/// Reset the drop counter (called from PHP before test)
#[php_function]
pub fn bailout_test_reset() {
    DROP_COUNTER.store(0, Ordering::SeqCst);
}

/// Get the current drop counter value
#[php_function]
pub fn bailout_test_get_counter() -> u32 {
    DROP_COUNTER.load(Ordering::SeqCst)
}

/// Create a `DropTracker` and then call a PHP callback that triggers `exit()`.
/// If the fix for issue #537 works, the destructor should be called
/// before the exit actually happens.
#[php_function]
pub fn bailout_test_with_callback(callback: ext_php_rs::types::ZendCallable) {
    let _tracker1 = DropTracker::new(1);
    let _tracker2 = DropTracker::new(2);
    let _tracker3 = DropTracker::new(3);

    // Call the PHP callback which will trigger exit()
    // try_call catches bailouts internally, so we need to check if it failed
    // and re-trigger the bailout manually
    let result = callback.try_call(vec![]);

    // If the callback triggered a bailout (exit/die/fatal error),
    // re-trigger it after our destructors have a chance to run.
    // The destructors will run when this function exits, before the
    // bailout is re-triggered by the handler wrapper.
    if result.is_err() {
        // Don't re-trigger here - let the handler wrapper do it
        // The handler wrapper's try_catch will see this as a normal return,
        // but our destructors will still run when this function's scope ends
    }
}

/// Create a `DropTracker` without bailout (control test)
#[php_function]
pub fn bailout_test_without_exit() {
    let _tracker1 = DropTracker::new(1);
    let _tracker2 = DropTracker::new(2);

    // No bailout - destructors should run normally when function returns
}

/// Test `BailoutGuard` - wrap resources that MUST be cleaned up in `BailoutGuard`.
/// This demonstrates using `BailoutGuard` for guaranteed cleanup even on direct bailout.
#[php_function]
pub fn bailout_test_with_guard(callback: ext_php_rs::types::ZendCallable) {
    // Wrap trackers in BailoutGuard - these will be cleaned up even if bailout
    // occurs directly (not caught by try_call)
    let _guarded1 = BailoutGuard::new(DropTracker::new(1));
    let _guarded2 = BailoutGuard::new(DropTracker::new(2));

    // This unguarded tracker demonstrates the difference - without BailoutGuard,
    // and without try_call catching the bailout, this would NOT be cleaned up.
    // But since try_call catches it, all destructors run normally.
    let _unguarded = DropTracker::new(3);

    // Call the PHP callback which will trigger exit()
    let _ = callback.try_call(vec![]);
}

/// Inner function for nested bailout test - creates guarded resources
fn nested_inner(callback: &ext_php_rs::types::ZendCallable) {
    let _inner_guard1 = BailoutGuard::new(DropTracker::new(10));
    let _inner_guard2 = BailoutGuard::new(DropTracker::new(11));

    // Call the PHP callback which will trigger exit()
    let _ = callback.try_call(vec![]);
}

/// Test nested calls with `BailoutGuard` - verifies cleanup happens at all nesting levels.
/// This creates guards at multiple call stack levels, then triggers bailout from the innermost.
#[php_function]
pub fn bailout_test_nested(callback: ext_php_rs::types::ZendCallable) {
    // Outer level guards
    let _outer_guard1 = BailoutGuard::new(DropTracker::new(1));
    let _outer_guard2 = BailoutGuard::new(DropTracker::new(2));

    // Call inner function which creates more guards and triggers bailout
    nested_inner(&callback);

    // This code won't be reached due to bailout, but the guards should still be cleaned up
}

/// Test deeply nested calls (3 levels) with `BailoutGuard`
#[php_function]
pub fn bailout_test_deep_nested(callback: ext_php_rs::types::ZendCallable) {
    // Level 1: outer guards
    let _level1_guard = BailoutGuard::new(DropTracker::new(1));

    // Level 2: call a closure that creates more guards
    let level2 = || {
        let _level2_guard = BailoutGuard::new(DropTracker::new(2));

        // Level 3: another closure
        let level3 = || {
            let _level3_guard = BailoutGuard::new(DropTracker::new(3));

            // Trigger bailout at the deepest level
            let _ = callback.try_call(vec![]);
        };

        level3();
    };

    level2();
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(bailout_test_reset))
        .function(wrap_function!(bailout_test_get_counter))
        .function(wrap_function!(bailout_test_with_callback))
        .function(wrap_function!(bailout_test_without_exit))
        .function(wrap_function!(bailout_test_with_guard))
        .function(wrap_function!(bailout_test_nested))
        .function(wrap_function!(bailout_test_deep_nested))
}

#[cfg(test)]
mod tests {
    #[test]
    fn bailout_destructor_called() {
        // First, run the control test (no bailout) to verify basic functionality
        assert!(crate::integration::test::run_php(
            "bailout/bailout_control.php"
        ));

        // Now run the bailout test - this verifies that destructors are called
        // even when a PHP callback triggers exit()
        assert!(crate::integration::test::run_php(
            "bailout/bailout_exit.php"
        ));

        // Test BailoutGuard cleanup mechanism
        assert!(crate::integration::test::run_php(
            "bailout/bailout_guard.php"
        ));

        // Test nested calls with BailoutGuard (2 levels)
        assert!(crate::integration::test::run_php(
            "bailout/bailout_nested.php"
        ));

        // Test deeply nested calls with BailoutGuard (3 levels via closures)
        assert!(crate::integration::test::run_php(
            "bailout/bailout_deep_nested.php"
        ));
    }
}
