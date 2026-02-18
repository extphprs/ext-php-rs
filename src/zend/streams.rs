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
    /// Get wrapped stream by name
    #[inline]
    #[must_use]
    pub fn get(name: &str) -> Option<&Self> {
        unsafe {
            let result = php_stream_locate_url_wrapper(name.as_ptr().cast(), ptr::null_mut(), 0);
            Some(NonNull::new(result)?.as_ref())
        }
    }

    /// Get mutable wrapped stream by name
    #[inline]
    #[must_use]
    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(name: &str) -> Option<&mut Self> {
        unsafe {
            let result = php_stream_locate_url_wrapper(name.as_ptr().cast(), ptr::null_mut(), 0);
            Some(NonNull::new(result)?.as_mut())
        }
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

impl StreamWrapperOps {}
