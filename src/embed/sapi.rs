//! Builder and objects for creating modules in PHP. A module is the base of a
//! PHP extension.

use std::ffi::CString;
use std::ptr;

use crate::ffi::sapi_module_struct;

/// A Zend module entry, also known as an extension.
pub type SapiModule = sapi_module_struct;

unsafe impl Send for SapiModule {}
unsafe impl Sync for SapiModule {}

impl SapiModule {
    /// Allocates the module entry on the heap, returning a pointer to the
    /// memory location. The caller is responsible for the memory pointed to.
    #[must_use]
    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }
}

/// Frees every string allocation that
/// [`SapiBuilder`](crate::builders::SapiBuilder) placed inside a
/// [`SapiModule`]: `name`, `pretty_name`, `executable_location`, `ini_entries`
/// and `php_ini_path_override`.
///
/// # Safety
///
/// * `module` must point to a valid [`SapiModule`] built by `SapiBuilder`.
///   Calling this on a module built by hand or by C is UB.
/// * Must be called **at most once**, and only after `sapi_shutdown`: PHP's
///   global `sapi_module` copy shares these pointers until then.
pub unsafe fn cleanup_sapi_allocations(module: *mut SapiModule) {
    let module = unsafe { &mut *module };

    if !module.name.is_null() {
        unsafe { drop(CString::from_raw(module.name)) };
        module.name = ptr::null_mut();
    }
    if !module.pretty_name.is_null() {
        unsafe { drop(CString::from_raw(module.pretty_name)) };
        module.pretty_name = ptr::null_mut();
    }
    if !module.executable_location.is_null() {
        unsafe { drop(CString::from_raw(module.executable_location)) };
        module.executable_location = ptr::null_mut();
    }
    if !module.ini_entries.is_null() {
        // Safety report from Clippy: `ini_entries` is `*mut c_char` on PHP
        // 8.1/8.2 and `*const c_char` on 8.3+, so an `as` cast is the only
        // spelling that compiles on every supported version.
        #[allow(clippy::ptr_cast_constness)]
        unsafe {
            drop(CString::from_raw(module.ini_entries as *mut _));
        }
        module.ini_entries = ptr::null_mut();
    }
    if !module.php_ini_path_override.is_null() {
        unsafe { drop(CString::from_raw(module.php_ini_path_override)) };
        module.php_ini_path_override = ptr::null_mut();
    }
}
