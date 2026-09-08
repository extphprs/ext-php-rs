//! Process-lifetime owner for tables the Zend engine borrows and never frees.
//!
//! A C extension puts its `zend_internal_arg_info[]` and `zend_ini_entry_def[]`
//! in `.rodata`, so the linker owns them for the life of the process.
//! ext-php-rs builds the same tables on the heap at MINIT, which leaves them
//! without an owner. This module is that owner.
//!
//! Nothing is ever handed back or freed. The list exists so the allocations
//! stay *reachable*, which is both what a leak detector needs to classify them
//! as still-reachable rather than lost, and what makes them correct under
//! `dl()`, ZTS, and a second `php_module_startup` in the same process.
//!
//! What the engine keeps, verified against php-src 8.1 through 8.5:
//!
//! * `zend_register_functions` copies the `arg_info` array only when the engine
//!   derives `ZEND_ACC_HAS_RETURN_TYPE` or `ZEND_ACC_HAS_TYPE_HINTS` from the
//!   types it carries, and that copy is shallow: `arg_info[i].name` and
//!   `.default_value` stay borrowed by pointer and are read at runtime by
//!   `ReflectionParameter`.
//! * `module_destructor` walks `module->functions` reading `fname` *after*
//!   `module_shutdown_func` returns for `dl()`-loaded modules, and
//!   `get_extension_funcs()` walks it during a request.
//! * PHP 8.5 stores `p->def = ini_entry` in `zend_register_ini_entries_ex`, and
//!   the CLI SAPI dereferences `def->value` for `php --ini=diff`.
//!
//! Registration runs once per process, so the list is bounded by the
//! extension's own surface.
// ponytail: an embedder looping MINIT/MSHUTDOWN/MINIT accumulates one set per
// cycle; add a generation id if that ever matters.

use std::ffi::{CString, c_char};

use parking_lot::{Mutex, const_mutex};

/// A pointer kept solely so the allocation behind it stays reachable.
struct Retained(#[expect(dead_code, reason = "held for reachability, never read")] *mut ());

// SAFETY: the pointer is stored for reachability only. It is never
// dereferenced, never handed back out and never freed, so moving one between
// threads cannot alias or race with the engine's use of the allocation.
unsafe impl Send for Retained {}

static RETAINED: Mutex<Vec<Retained>> = const_mutex(Vec::new());

/// Gives `value` to the Zend engine for the remainder of the process.
pub(crate) fn retain<T: ?Sized>(value: Box<T>) -> *mut T {
    let ptr = Box::into_raw(value);
    RETAINED.lock().push(Retained(ptr.cast::<()>()));
    ptr
}

/// Gives `value`'s buffer to the Zend engine for the remainder of the process.
pub(crate) fn cstring(value: CString) -> *mut c_char {
    retain(value.into_boxed_c_str()).cast::<c_char>()
}
