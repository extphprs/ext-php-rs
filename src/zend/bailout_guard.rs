//! Provides cleanup guarantees for values that need to be dropped even when PHP bailout occurs.
//!
//! When PHP triggers a bailout (via `exit()`, fatal error, etc.), it uses `longjmp` which
//! bypasses Rust's normal stack unwinding. This means destructors for stack-allocated values
//! won't run. `BailoutGuard` solves this by heap-allocating values and registering cleanup
//! callbacks that run when a bailout is caught.
//!
//! # Example
//!
//! ```ignore
//! use ext_php_rs::zend::BailoutGuard;
//!
//! #[php_function]
//! pub fn my_function(callback: ZendCallable) {
//!     // Wrap resources that MUST be cleaned up in BailoutGuard
//!     let resource = BailoutGuard::new(ExpensiveResource::new());
//!
//!     // Use the resource (BailoutGuard implements Deref/DerefMut)
//!     resource.do_something();
//!
//!     // If the callback triggers exit(), the resource will still be cleaned up
//!     let _ = callback.try_call(vec![]);
//! }
//! ```

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

/// A cleanup entry: (callback, active). The active flag is set to false when
/// the guard is dropped normally, so we don't double-drop.
type CleanupEntry = (Box<dyn FnOnce()>, bool);

thread_local! {
    /// Stack of cleanup callbacks to run when bailout is caught.
    static CLEANUP_STACK: RefCell<Vec<CleanupEntry>> = const { RefCell::new(Vec::new()) };
}

/// A guard that ensures a value is dropped even if PHP bailout occurs.
///
/// `BailoutGuard` heap-allocates the wrapped value and registers a cleanup callback.
/// If a bailout occurs, the cleanup runs before the bailout is re-triggered.
/// If the guard is dropped normally, the cleanup is cancelled and the value is dropped.
///
/// # Performance Note
///
/// This incurs a heap allocation. Only use for values that absolutely must be
/// cleaned up (file handles, network connections, locks, etc.). For simple values,
/// the overhead isn't worth it.
pub struct BailoutGuard<T> {
    /// Pointer to the heap-allocated value. Using raw pointer because we need
    /// to pass it to the cleanup callback.
    value: *mut T,
    /// Index in the cleanup stack. Used to deactivate cleanup on normal drop.
    index: usize,
}

// SAFETY: BailoutGuard can be sent between threads if T can.
// The cleanup stack is thread-local, so each thread has its own.
unsafe impl<T: Send> Send for BailoutGuard<T> {}

impl<T: 'static> BailoutGuard<T> {
    /// Creates a new `BailoutGuard` wrapping the given value.
    ///
    /// The value is heap-allocated and a cleanup callback is registered.
    /// If a bailout occurs, the value will be dropped. If this guard is
    /// dropped normally, the value is dropped and the cleanup is cancelled.
    pub fn new(value: T) -> Self {
        let boxed = Box::new(value);
        let ptr = Box::into_raw(boxed);

        let index = CLEANUP_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            let idx = stack.len();
            let ptr_copy = ptr;
            // Register cleanup that drops the heap-allocated value
            stack.push((
                Box::new(move || {
                    // SAFETY: This only runs if bailout occurred and normal drop didn't.
                    // The pointer is valid because we heap-allocated it.
                    unsafe {
                        drop(Box::from_raw(ptr_copy));
                    }
                }),
                true, // active
            ));
            idx
        });

        Self { value: ptr, index }
    }

    /// Returns a reference to the wrapped value.
    #[inline]
    #[must_use]
    pub fn get(&self) -> &T {
        // SAFETY: The pointer is valid as long as self exists.
        unsafe { &*self.value }
    }

    /// Returns a mutable reference to the wrapped value.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: The pointer is valid as long as self exists, and we have &mut self.
        unsafe { &mut *self.value }
    }

    /// Consumes the guard and returns the wrapped value.
    ///
    /// The cleanup callback is cancelled.
    #[must_use]
    pub fn into_inner(self) -> T {
        // Deactivate cleanup
        CLEANUP_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            if self.index < stack.len() {
                stack[self.index].1 = false;
            }
        });

        // Take ownership of the value
        // SAFETY: We're consuming self, so no one else can access the pointer.
        let value = unsafe { *Box::from_raw(self.value) };

        // Prevent Drop from running (we've already handled cleanup)
        std::mem::forget(self);

        value
    }
}

impl<T> Deref for BailoutGuard<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        // SAFETY: The pointer is valid as long as self exists.
        unsafe { &*self.value }
    }
}

impl<T> DerefMut for BailoutGuard<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: The pointer is valid as long as self exists, and we have &mut self.
        unsafe { &mut *self.value }
    }
}

impl<T> Drop for BailoutGuard<T> {
    fn drop(&mut self) {
        // Deactivate cleanup callback (we're dropping normally)
        CLEANUP_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            if self.index < stack.len() {
                stack[self.index].1 = false;
            }
        });

        // Drop the heap-allocated value
        // SAFETY: We're in Drop, so no one else can access the pointer.
        unsafe {
            drop(Box::from_raw(self.value));
        }
    }
}

/// Runs all registered bailout cleanup callbacks.
///
/// This should be called after catching a bailout and before re-triggering it.
/// Only active cleanups (those whose guards haven't been dropped) are run.
///
/// # Note
///
/// This function is automatically called by the generated handler code when a
/// bailout is caught. You typically don't need to call this directly.
#[doc(hidden)]
pub fn run_bailout_cleanups() {
    CLEANUP_STACK.with(|stack| {
        // Drain and run all active cleanups in reverse order (LIFO)
        for (cleanup, active) in stack.borrow_mut().drain(..).rev() {
            if active {
                cleanup();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Creates a drop counter that increments the given `AtomicUsize` on drop.
    fn make_drop_counter(counter: Arc<AtomicUsize>) -> impl Drop + 'static {
        struct DropCounter(Arc<AtomicUsize>);
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        DropCounter(counter)
    }

    #[test]
    fn test_normal_drop() {
        let drop_count = Arc::new(AtomicUsize::new(0));
        // Clear any leftover cleanup entries from previous tests
        CLEANUP_STACK.with(|stack| stack.borrow_mut().clear());

        {
            let _guard = BailoutGuard::new(make_drop_counter(Arc::clone(&drop_count)));
            assert_eq!(drop_count.load(Ordering::SeqCst), 0);
        }

        // Value should be dropped when guard goes out of scope
        assert_eq!(drop_count.load(Ordering::SeqCst), 1);

        // Cleanup stack should be empty (cleanup was deactivated)
        CLEANUP_STACK.with(|stack| {
            assert!(stack.borrow().is_empty() || !stack.borrow()[0].1);
        });
    }

    #[test]
    fn test_bailout_cleanup() {
        let drop_count = Arc::new(AtomicUsize::new(0));
        // Clear any leftover cleanup entries from previous tests
        CLEANUP_STACK.with(|stack| stack.borrow_mut().clear());

        // Simulate what happens during bailout:
        // 1. Guard is created
        // 2. Bailout occurs (longjmp) - guard's Drop doesn't run
        // 3. run_bailout_cleanups() is called

        let guard = BailoutGuard::new(make_drop_counter(Arc::clone(&drop_count)));

        // Simulate bailout - don't drop the guard normally
        std::mem::forget(guard);

        // Value hasn't been dropped yet
        assert_eq!(drop_count.load(Ordering::SeqCst), 0);

        // Run bailout cleanups (simulating what try_catch does)
        run_bailout_cleanups();

        // Value should now be dropped
        assert_eq!(drop_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_into_inner() {
        let drop_count = Arc::new(AtomicUsize::new(0));
        // Clear any leftover cleanup entries from previous tests
        CLEANUP_STACK.with(|stack| stack.borrow_mut().clear());

        let guard = BailoutGuard::new(make_drop_counter(Arc::clone(&drop_count)));
        let value = guard.into_inner();

        // Value hasn't been dropped yet (we own it now)
        assert_eq!(drop_count.load(Ordering::SeqCst), 0);

        drop(value);

        // Now it's dropped
        assert_eq!(drop_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_deref() {
        let guard = BailoutGuard::new(String::from("hello"));
        assert_eq!(&*guard, "hello");
        assert_eq!(guard.len(), 5);
    }

    #[test]
    fn test_deref_mut() {
        let mut guard = BailoutGuard::new(String::from("hello"));
        guard.push_str(" world");
        assert_eq!(&*guard, "hello world");
    }
}
