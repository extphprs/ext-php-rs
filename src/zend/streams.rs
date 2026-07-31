use std::ffi::CString;
use std::ptr::{self, NonNull};

use crate::{
    error::Error,
    ffi::{
        php_register_url_stream_wrapper, php_register_url_stream_wrapper_volatile, php_stream,
        php_stream_context, php_stream_locate_url_wrapper, php_stream_wrapper,
        php_stream_wrapper_ops, php_unregister_url_stream_wrapper,
        php_unregister_url_stream_wrapper_volatile, zend_string,
    },
    types::ZendStr,
};

/// Wrapper for PHP streams
pub type StreamWrapper = php_stream_wrapper;

/// Stream opener function
pub type StreamOpener = unsafe extern "C" fn(
    *mut StreamWrapper,
    *const std::ffi::c_char,
    *const std::ffi::c_char,
    i32,
    *mut *mut zend_string,
    *mut php_stream_context,
    i32,
    *const std::ffi::c_char,
    u32,
    *const std::ffi::c_char,
    u32,
) -> *mut Stream;

impl StreamWrapper {
    /// Locates the stream wrapper registered for `name`.
    ///
    /// # Safety
    ///
    /// The returned wrapper is owned by PHP and lives in the per-request stream
    /// wrapper table, so the caller must not hold `'a` beyond the current
    /// request. The lifetime is unconstrained by the arguments and must be
    /// chosen to reflect that.
    #[inline]
    #[must_use]
    pub unsafe fn get<'a>(name: &str) -> Option<&'a Self> {
        let ptr = Self::locate(name)?;
        // SAFETY: `locate` returned a non-null wrapper pointer owned by PHP, and
        // the caller guarantees `'a` does not outlive the current request.
        Some(unsafe { ptr.as_ref() })
    }

    /// Locates the stream wrapper registered for `name` for mutation.
    ///
    /// # Safety
    ///
    /// In addition to the request-scoped validity required by [`Self::get`], the
    /// caller must ensure no other reference to the same wrapper is live for
    /// `'a`. The wrapper table is process-wide state shared with PHP and with
    /// every other extension, so this function cannot check exclusivity.
    #[inline]
    #[must_use]
    pub unsafe fn get_mut<'a>(name: &str) -> Option<&'a mut Self> {
        let mut ptr = Self::locate(name)?;
        // SAFETY: `locate` returned a non-null wrapper pointer owned by PHP, and
        // the caller guarantees both request-scoped validity and exclusivity.
        Some(unsafe { ptr.as_mut() })
    }

    /// Looks up the wrapper pointer for `name`.
    ///
    /// `name` is copied into a NUL-terminated C string because
    /// `php_stream_locate_url_wrapper` scans the path until it reaches a
    /// character outside `[A-Za-z0-9+-.]`, which would read past the end of a
    /// non-terminated Rust string slice.
    #[inline]
    fn locate(name: &str) -> Option<NonNull<Self>> {
        let name = CString::new(name).ok()?;
        // SAFETY: `name` is a valid NUL-terminated C string that outlives the
        // call, and a null `path_for_open` is accepted by PHP.
        let result = unsafe { php_stream_locate_url_wrapper(name.as_ptr(), ptr::null_mut(), 0) };
        NonNull::new(result)
    }

    /// Register stream wrapper for name
    ///
    /// # Errors
    ///
    /// * `Error::StreamWrapperRegistrationFailure` - If the stream wrapper
    ///   could not be registered
    ///
    /// # Panics
    ///
    /// * If the name cannot be converted to a C string
    pub fn register(self, name: &str) -> Result<Self, Error> {
        // We have to convert it to a static so owned streamwrapper doesn't get dropped.
        let copy = Box::new(self);
        let copy = Box::leak(copy);
        let name = std::ffi::CString::new(name).expect("Could not create C string for name!");
        let result = unsafe { php_register_url_stream_wrapper(name.as_ptr(), copy) };
        if result == 0 {
            Ok(*copy)
        } else {
            Err(Error::StreamWrapperRegistrationFailure)
        }
    }

    /// Register volatile stream wrapper for name
    ///
    /// # Errors
    ///
    /// * `Error::StreamWrapperRegistrationFailure` - If the stream wrapper
    ///   could not be registered
    pub fn register_volatile(self, name: &str) -> Result<Self, Error> {
        // We have to convert it to a static so owned streamwrapper doesn't get dropped.
        let copy = Box::new(self);
        let copy = Box::leak(copy);
        let name = ZendStr::new(name, false);
        let result =
            unsafe { php_register_url_stream_wrapper_volatile((*name).as_ptr().cast_mut(), copy) };
        if result == 0 {
            Ok(*copy)
        } else {
            Err(Error::StreamWrapperRegistrationFailure)
        }
    }

    /// Unregister stream wrapper by name
    ///
    /// # Errors
    ///
    /// * `Error::StreamWrapperUnregistrationFailure` - If the stream wrapper
    ///   could not be unregistered
    ///
    /// # Panics
    ///
    /// * If the name cannot be converted to a C string
    pub fn unregister(name: &str) -> Result<(), Error> {
        let name = std::ffi::CString::new(name).expect("Could not create C string for name!");
        match unsafe { php_unregister_url_stream_wrapper(name.as_ptr()) } {
            0 => Ok(()),
            _ => Err(Error::StreamWrapperUnregistrationFailure),
        }
    }

    /// Unregister volatile stream wrapper by name
    ///
    /// # Errors
    ///
    /// * `Error::StreamWrapperUnregistrationFailure` - If the stream wrapper
    ///   could not be unregistered
    pub fn unregister_volatile(name: &str) -> Result<(), Error> {
        let name = ZendStr::new(name, false);
        match unsafe { php_unregister_url_stream_wrapper_volatile((*name).as_ptr().cast_mut()) } {
            0 => Ok(()),
            _ => Err(Error::StreamWrapperUnregistrationFailure),
        }
    }

    /// Get the operations the stream wrapper can perform
    #[must_use]
    pub fn wops(&self) -> &php_stream_wrapper_ops {
        unsafe { &*self.wops }
    }

    /// Get the mutable operations the stream can perform
    pub fn wops_mut(&mut self) -> &mut php_stream_wrapper_ops {
        unsafe { &mut *(self.wops.cast_mut()) }
    }
}

/// A PHP stream
pub type Stream = php_stream;

/// Operations that can be performed with a stream wrapper
pub type StreamWrapperOps = php_stream_wrapper_ops;
