//! Builder and objects for creating modules in PHP. A module is the base of a
//! PHP extension.

use std::cell::UnsafeCell;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::ptr;
use std::sync::Once;

use crate::ffi::zend_module_entry;

fn zend_type_has_name(type_mask: u32) -> bool {
    cfg_if::cfg_if! {
        if #[cfg(php83)] {
            (type_mask & crate::ffi::_ZEND_TYPE_LITERAL_NAME_BIT) != 0
        } else {
            (type_mask & crate::ffi::_ZEND_TYPE_NAME_BIT) != 0
        }
    }
}

/// A Zend module entry, also known as an extension.
pub type ModuleEntry = zend_module_entry;

impl ModuleEntry {
    /// Allocates the module entry on the heap, returning a pointer to the
    /// memory location. The caller is responsible for the memory pointed to.
    #[deprecated(note = "use StaticModuleEntry to avoid leaking the allocation")]
    #[must_use]
    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }
}

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

/// Frees every heap allocation that ext-php-rs placed inside a
/// [`ModuleEntry`]: the `name`/`version` `CString`s, the `functions` boxed
/// slice, and all nested `fname`/`arg_info`/`default_value`/class-name
/// pointers.
///
/// # Safety
///
/// * Must be called **exactly once**, during MSHUTDOWN, **before** PHP calls
///   `DL_UNLOAD`.
/// * All pointer fields must originate from ext-php-rs (`CString::into_raw` /
///   `Box::into_raw`). Calling this on a module built by hand or by C is UB.
pub unsafe fn cleanup_module_allocations(entry: *mut ModuleEntry) {
    let entry = unsafe { &mut *entry };

    if !entry.name.is_null() {
        unsafe { drop(CString::from_raw(entry.name.cast_mut())) };
        entry.name = ptr::null();
    }
    if !entry.version.is_null() {
        unsafe { drop(CString::from_raw(entry.version.cast_mut())) };
        entry.version = ptr::null();
    }

    if entry.functions.is_null() {
        return;
    }

    let funcs = entry.functions.cast_mut();
    let mut count: usize = 0;

    while !unsafe { (*funcs.add(count)).fname }.is_null() {
        let func = unsafe { &mut *funcs.add(count) };

        unsafe { drop(CString::from_raw(func.fname.cast_mut())) };
        func.fname = ptr::null();

        // arg_info[0].name is `required_num_args` cast to a pointer, not a CString.
        if !func.arg_info.is_null() {
            let n = func.num_args as usize;
            let base = func.arg_info.cast_mut();

            for i in 0..=n {
                let arg = unsafe { &mut *base.add(i) };

                if i > 0 && !arg.name.is_null() {
                    unsafe { drop(CString::from_raw(arg.name.cast_mut())) };
                }
                if !arg.default_value.is_null() {
                    unsafe { drop(CString::from_raw(arg.default_value.cast_mut())) };
                }
                if !arg.type_.ptr.is_null() && zend_type_has_name(arg.type_.type_mask) {
                    unsafe { drop(CString::from_raw(arg.type_.ptr.cast::<c_char>())) };
                }
            }

            unsafe { drop(Box::from_raw(ptr::slice_from_raw_parts_mut(base, n + 1))) };
        }

        count += 1;
    }

    unsafe {
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(
            funcs,
            count + 1,
        )));
    }
    entry.functions = ptr::null();
}
