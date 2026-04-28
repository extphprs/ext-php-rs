//! Safe wrappers around the raw `zend_string` allocation FFI used by type
//! registration paths (e.g. class union members).
//!
//! Feature modules consume these wrappers; they should not call the underlying
//! `ffi::zend_string_init_interned` directly.

#[cfg(not(php83))]
use crate::ffi::zend_string;
#[cfg(not(php83))]
use crate::types::ZendStr;

/// Interns `name` as a persistent `zend_string` and returns the raw pointer.
///
/// "Persistent" here matches Zend's `pemalloc(_, 1)` discipline: the string is
/// allocated outside the request-bound heap and lives for the lifetime of the
/// engine, so the returned pointer remains valid across requests. This is the
/// shape required for entries inside a `zend_type_list` (see
/// `Zend/zend_API.c:2962-2964` in php-src).
///
/// Ownership is transferred to the engine's interned-string table; the caller
/// must not free the result. Subsequent calls with the same content will
/// dedupe through Zend's interning.
///
/// Returns [`None`] if the engine has not yet wired up
/// `zend_string_init_interned` (i.e. before `zend_startup`).
///
/// # Panics
///
/// Panics if the engine returns a null pointer (out-of-memory inside Zend's
/// allocator), matching [`ZendStr::new_interned`]'s contract.
#[cfg(not(php83))]
pub(crate) fn intern_persistent(name: &str) -> *mut zend_string {
    let boxed = ZendStr::new_interned(name, true);
    let zstr: &'static mut ZendStr = crate::boxed::ZBox::into_raw(boxed);
    std::ptr::from_mut(zstr)
}
