//! Drops a value even when a PHP bailout skips its frame.
//!
//! A bailout is a `longjmp` that the engine makes on a fatal error or on `memory_limit`
//! exhaustion. The jump skips the Rust frames between the error and the catch point, so
//! Rust does not run their destructors. `BailoutGuard` moves the value to the heap and
//! registers a cleanup entry with the innermost [`try_catch`](crate::zend::try_catch)
//! frame. That frame drops the value when it catches the bailout.
//!
//! # Ownership
//!
//! Every [`try_catch`](crate::zend::try_catch) call records the depth of the cleanup stack
//! on entry. The wrapper of each exported PHP function is such a call. When the frame
//! catches a bailout, it drops the guards that were created inside its closure, newest
//! first. The guards that were created before the frame are not changed. You can still
//! use them.
//!
//! Do not move a guard out of a `try_catch` closure through shared mutable state, for
//! example `AssertUnwindSafe(&mut Option<_>)`. If that closure bails out, the frame drops
//! the value and the moved guard points at freed memory.
//!
//! `BailoutGuard` is `!Send`. The cleanup stack belongs to the thread that created the
//! guard.
//!
//! # Example
//!
//! ```ignore
//! use ext_php_rs::zend::BailoutGuard;
//!
//! #[php_function]
//! pub fn my_function(callback: ZendCallable) {
//!     let resource = BailoutGuard::new(ExpensiveResource::new());
//!
//!     // BailoutGuard implements Deref and DerefMut
//!     resource.do_something();
//!
//!     // The resource is dropped even if the callback hits a fatal error
//!     let _ = callback.try_call(vec![]);
//! }
//! ```
//!
//! A guard cannot leave its thread:
//!
//! ```compile_fail
//! use ext_php_rs::zend::BailoutGuard;
//!
//! let guard = BailoutGuard::new(42_u64);
//! std::thread::spawn(move || drop(guard));
//! ```

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

/// A cleanup entry. `None` marks an entry whose guard was released out of LIFO
/// order; it is popped once every newer entry is gone.
type CleanupEntry = Option<Box<dyn FnOnce()>>;

thread_local! {
    static CLEANUP_STACK: RefCell<Vec<CleanupEntry>> = const { RefCell::new(Vec::new()) };
}

/// A guard that drops its value even if a PHP bailout skips its frame.
///
/// `BailoutGuard::new` moves the value to the heap and registers a cleanup entry with
/// the innermost [`try_catch`](crate::zend::try_catch) frame. If that frame catches a
/// bailout, it drops the value. If you drop the guard, the guard releases the entry and
/// drops the value.
///
/// # Performance
///
/// `BailoutGuard::new` makes one heap allocation. Use it only for values that must be
/// released: file handles, network connections, locks. Do not wrap simple values.
pub struct BailoutGuard<T> {
    value: *mut T,
    index: usize,
}

impl<T: 'static> BailoutGuard<T> {
    /// Wraps `value` in a new guard.
    ///
    /// The value moves to the heap. The guard registers a cleanup entry with the
    /// innermost `try_catch` frame.
    pub fn new(value: T) -> Self {
        let ptr = Box::into_raw(Box::new(value));
        let index = CLEANUP_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            stack.push(Some(Box::new(move || {
                // SAFETY: only the bailout path runs this closure. The frames that owned
                // the guard were jumped over by longjmp, so its Drop never runs and this is
                // the only release of the allocation.
                unsafe { drop(Box::from_raw(ptr)) }
            })));
            stack.len() - 1
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

    /// Consumes the guard and returns the value.
    ///
    /// The guard releases its cleanup entry.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.release();
        // SAFETY: We're consuming self, so no one else can access the pointer.
        let value = unsafe { *Box::from_raw(self.value) };
        std::mem::forget(self);
        value
    }
}

impl<T> BailoutGuard<T> {
    fn release(&self) {
        CLEANUP_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            if self.index + 1 == stack.len() {
                stack.pop();
                while stack.last().is_some_and(Option::is_none) {
                    stack.pop();
                }
            } else if let Some(entry) = stack.get_mut(self.index) {
                *entry = None;
            }
        });
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
        self.release();
        // SAFETY: We're in Drop, so no one else can access the pointer.
        unsafe { drop(Box::from_raw(self.value)) }
    }
}

/// Current depth of the cleanup stack, recorded by `try_catch` on entry.
pub(crate) fn cleanup_depth() -> usize {
    CLEANUP_STACK.with(|stack| stack.borrow().len())
}

/// Removes the cleanup entries above `depth` and runs them, newest first.
///
/// `try_catch` calls this after it catches a bailout. The entries leave the stack before
/// they run, so a destructor can create new guards or call the engine.
pub(crate) fn run_cleanups_above(depth: usize) {
    let entries: Vec<CleanupEntry> = CLEANUP_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        if depth >= stack.len() {
            return Vec::new();
        }
        stack.drain(depth..).collect()
    });
    for cleanup in entries.into_iter().rev().flatten() {
        cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct DropCounter(Arc<AtomicUsize>);
    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn counter() -> (Arc<AtomicUsize>, impl Fn() -> DropCounter) {
        let count = Arc::new(AtomicUsize::new(0));
        let make = {
            let count = Arc::clone(&count);
            move || DropCounter(Arc::clone(&count))
        };
        (count, make)
    }

    fn reset() {
        CLEANUP_STACK.with(|stack| stack.borrow_mut().clear());
    }

    #[test]
    fn normal_drop_pops_entry() {
        reset();
        let (count, make) = counter();
        {
            let _guard = BailoutGuard::new(make());
            assert_eq!(cleanup_depth(), 1);
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(cleanup_depth(), 0);
    }

    #[test]
    fn out_of_order_drop_drains_once_newer_guard_goes() {
        reset();
        let (count, make) = counter();
        let older = BailoutGuard::new(make());
        let newer = BailoutGuard::new(make());
        drop(older);
        assert_eq!(cleanup_depth(), 2);
        drop(newer);
        assert_eq!(cleanup_depth(), 0);
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn cleanups_above_depth_spare_outer_guard() {
        reset();
        let (count, make) = counter();
        let outer = BailoutGuard::new(make());
        let depth = cleanup_depth();
        let inner = BailoutGuard::new(make());
        std::mem::forget(inner);

        run_cleanups_above(depth);

        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(cleanup_depth(), 1);
        assert_eq!(outer.get().0.load(Ordering::SeqCst), 1);
        drop(outer);
        assert_eq!(count.load(Ordering::SeqCst), 2);
        assert_eq!(cleanup_depth(), 0);
    }

    struct Record(Arc<std::sync::Mutex<Vec<u8>>>, u8);
    impl Drop for Record {
        fn drop(&mut self) {
            self.0.lock().expect("poisoned").push(self.1);
        }
    }

    #[test]
    fn cleanups_run_newest_first() {
        reset();
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        std::mem::forget(BailoutGuard::new(Record(Arc::clone(&order), 1)));
        std::mem::forget(BailoutGuard::new(Record(Arc::clone(&order), 2)));

        run_cleanups_above(0);

        assert_eq!(*order.lock().expect("poisoned"), vec![2, 1]);
    }

    #[test]
    fn into_inner_releases_entry() {
        reset();
        let (count, make) = counter();
        let value = BailoutGuard::new(make()).into_inner();
        assert_eq!(cleanup_depth(), 0);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        drop(value);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn deref_and_deref_mut() {
        let mut guard = BailoutGuard::new(String::from("hello"));
        guard.push_str(" world");
        assert_eq!(&*guard, "hello world");
        assert_eq!(guard.len(), 11);
    }
}
