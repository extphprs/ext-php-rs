//! A Zend bailout while [`PhpException::throw`] runs must not leak the Rust
//! strings it owns.
//!
//! Outside a PHP frame, `zend_throw_exception_internal` always bails out
//! ("Exception thrown without a stack frame"), which jumps over `throw`'s frame.
//! `try_catch` then runs the `BailoutGuard` cleanups, as the generated handlers
//! do. This binary counts the bytes Rust has outstanding, so a leaked buffer
//! shows up on every platform, without a leak sanitizer.
#![cfg(feature = "embed")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use ext_php_rs::convert::IntoZval;
use ext_php_rs::embed::Embed;
use ext_php_rs::exception::PhpException;
use ext_php_rs::types::ZendObject;
use ext_php_rs::zend::{ce, try_catch};

struct Counting;

static OUTSTANDING: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is forwarded to `System` with the caller's arguments.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            OUTSTANDING.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        OUTSTANDING.fetch_sub(layout.size(), Ordering::SeqCst);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new = unsafe { System.realloc(ptr, layout, new_size) };
        if !new.is_null() {
            OUTSTANDING.fetch_add(new_size, Ordering::SeqCst);
            OUTSTANDING.fetch_sub(layout.size(), Ordering::SeqCst);
        }
        new
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Large enough that a leaked message cannot hide among small allocations.
const MESSAGE_LEN: usize = 1 << 20;

/// Bytes still outstanding after `exception().throw()` bails out. The first
/// round grows the cleanup stack to its working size; only the second counts.
fn leaked_by_bailout(exception: fn() -> PhpException) -> usize {
    Embed::run(|| {
        let mut leaked = 0;
        for _ in 0..2 {
            let before = OUTSTANDING.load(Ordering::SeqCst);
            let result = try_catch(|| exception().throw());
            assert!(result.is_err(), "throw outside a frame must bail out");
            leaked = OUTSTANDING.load(Ordering::SeqCst).saturating_sub(before);
        }
        leaked
    })
}

// One test: the counter is process-wide, so the cases must not run in parallel.
#[test]
fn bailout_while_throwing_drops_the_rust_strings() {
    let message = leaked_by_bailout(|| PhpException::from_message("m".repeat(MESSAGE_LEN)));
    assert_eq!(message, 0, "message");

    // An interface can't be thrown, so `throw` formats a fallback `Error` and
    // throws that, with the original message escaped into it.
    let fallback = leaked_by_bailout(|| {
        PhpException::new("m\0".repeat(MESSAGE_LEN / 2), 0, ce::arrayaccess())
    });
    assert_eq!(fallback, 0, "fallback");

    let object = leaked_by_bailout(|| {
        let object = ZendObject::new(ce::exception())
            .into_zval(false)
            .expect("an exception object converts to a zval");
        PhpException::from_message("m".repeat(MESSAGE_LEN)).with_object(object)
    });
    assert_eq!(object, 0, "attached object");
}
