//! Builder and objects for creating modules in PHP. A module is the base of a
//! PHP extension.

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Once;

use crate::ffi::zend_module_entry;

/// A Zend module entry, also known as an extension.
pub type ModuleEntry = zend_module_entry;

/// Static storage for a [`ModuleEntry`] that avoids heap allocation.
///
/// Mimics how C extensions declare a `static zend_module_entry`. The entry
/// lives in the shared library's data segment and is reclaimed automatically
/// when PHP calls `DL_UNLOAD`.
pub struct StaticModuleEntry {
    init: Once,
    inner: UnsafeCell<MaybeUninit<ModuleEntry>>,
}

unsafe impl Sync for StaticModuleEntry {}

impl Default for StaticModuleEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl StaticModuleEntry {
    /// Creates a new uninitialized static module entry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            init: Once::new(),
            inner: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Initialises the entry on first call, returning a stable `*mut` pointer.
    ///
    /// Subsequent calls skip `f` and return the same pointer.
    pub fn get_or_init(&self, f: impl FnOnce() -> ModuleEntry) -> *mut ModuleEntry {
        self.init.call_once(|| unsafe {
            (*self.inner.get()).write(f());
        });
        unsafe { (*self.inner.get()).as_mut_ptr() }
    }
}

