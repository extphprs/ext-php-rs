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

/// Frees the string allocations that
/// [`SapiBuilder`](crate::builders::SapiBuilder) placed inside a
/// [`SapiModule`]: `name`, `pretty_name`, `executable_location` and
/// `php_ini_path_override`. PHP only reads these and never frees them.
///
/// `ini_entries` is left untouched: `sapi_startup` clears it before copying
/// the module, so after startup it holds whatever the embedder assigned
/// (typically an [`IniBuilder`](crate::builders::IniBuilder) buffer) and stays
/// theirs to free.
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
    if !module.php_ini_path_override.is_null() {
        unsafe { drop(CString::from_raw(module.php_ini_path_override)) };
        module.php_ini_path_override = ptr::null_mut();
    }
}
