//! # `cargo-php` CLI
//!
//! Installs extensions and generates stub files for PHP extensions generated
//! with `ext-php-rs`. Use `cargo php --help` for more information.

use std::ffi::c_void;
use std::os::raw::c_char;

/// Mock macro for the `allowed_bindings.rs` script.
/// Excludes functions that need real implementations for stub generation.
#[cfg(not(windows))]
macro_rules! bind {
    ($($s: ident),*) => {
        $(
            bind!(@INTERNAL; $s);
        )*
    };
    // Skip allocator functions - we provide real implementations below
    (@INTERNAL; _emalloc) => {};
    (@INTERNAL; _efree) => {};
    (@INTERNAL; _estrdup) => {};
    // For everything else, use null stub
    (@INTERNAL; $s: ident) => {
        cargo_php::stub_symbols!($s);
    };
}

#[cfg(not(windows))]
include!("../allowed_bindings.rs");

// Real implementations for PHP allocator functions used during stub generation.
// These use the system allocator since PHP's memory manager isn't available.

/// Stub implementation of PHP's _emalloc using system malloc.
///
/// # Safety
///
/// This function is unsafe because it directly calls libc malloc.
/// The caller must ensure proper memory management of the returned pointer.
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _emalloc(size: usize) -> *mut c_void {
    unsafe { libc::malloc(size) }
}

/// Stub implementation of PHP's _efree using system free.
///
/// # Safety
///
/// This function is unsafe because it directly calls libc free.
/// The caller must ensure the pointer was allocated by `_emalloc` or compatible allocator.
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _efree(ptr: *mut c_void) {
    unsafe { libc::free(ptr) }
}

/// Stub implementation of PHP's _estrdup using system strdup.
///
/// # Safety
///
/// This function is unsafe because it directly calls libc strdup.
/// The caller must ensure `s` is a valid null-terminated C string.
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _estrdup(s: *const c_char) -> *mut c_char {
    unsafe { libc::strdup(s) }
}

fn main() -> cargo_php::CrateResult {
    cargo_php::run()
}
