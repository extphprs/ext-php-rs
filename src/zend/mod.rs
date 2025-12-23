//! Types used to interact with the Zend engine.

mod _type;
mod bailout_guard;
pub mod ce;
mod class;
mod ex;
mod function;
mod globals;
mod handlers;
mod ini_entry_def;
mod linked_list;
mod module;
mod streams;
mod try_catch;

use crate::{
    error::Result,
    ffi::{php_output_write, php_printf, sapi_module},
};
use std::ffi::CString;
use std::os::raw::c_char;

pub use _type::ZendType;
pub use bailout_guard::BailoutGuard;
pub use bailout_guard::run_bailout_cleanups;
pub use class::ClassEntry;
pub use ex::ExecuteData;
pub use function::Function;
pub use function::FunctionEntry;
pub use globals::ExecutorGlobals;
pub use globals::FileGlobals;
pub use globals::ProcessGlobals;
pub use globals::SapiGlobals;
pub use globals::SapiHeader;
pub use globals::SapiHeaders;
pub use globals::SapiModule;
pub use handlers::ZendObjectHandlers;
pub use ini_entry_def::IniEntryDef;
pub use linked_list::ZendLinkedList;
pub use module::ModuleEntry;
pub use streams::*;
#[cfg(feature = "embed")]
pub(crate) use try_catch::panic_wrapper;
pub use try_catch::{CatchError, bailout, try_catch, try_catch_first};

// Used as the format string for `php_printf`.
const FORMAT_STR: &[u8] = b"%s\0";

/// Prints to stdout using the `php_printf` function.
///
/// Also see the [`php_print`] and [`php_println`] macros.
///
/// # Arguments
///
/// * message - The message to print to stdout.
///
/// # Errors
///
/// * If the message could not be converted to a [`CString`].
pub fn printf(message: &str) -> Result<()> {
    let message = CString::new(message)?;
    unsafe {
        php_printf(FORMAT_STR.as_ptr().cast(), message.as_ptr());
    };
    Ok(())
}

/// Writes binary data to PHP's output stream (stdout).
///
/// Unlike [`printf`], this function is binary-safe and can handle data
/// containing NUL bytes. It uses the SAPI module's `ub_write` function
/// which accepts a pointer and length, allowing arbitrary binary data.
///
/// Also see the [`php_write!`] macro.
///
/// # Arguments
///
/// * `data` - The binary data to write to stdout.
///
/// # Returns
///
/// The number of bytes written.
///
/// # Errors
///
/// Returns [`crate::error::Error::SapiWriteUnavailable`] if the SAPI's `ub_write` function
/// is not available.
///
/// # Example
///
/// ```ignore
/// use ext_php_rs::zend::write;
///
/// // Write binary data including NUL bytes
/// let data = b"Hello\x00World";
/// write(data).expect("Failed to write data");
/// ```
pub fn write(data: &[u8]) -> Result<usize> {
    unsafe {
        if let Some(ub_write) = sapi_module.ub_write {
            Ok(ub_write(data.as_ptr().cast::<c_char>(), data.len()))
        } else {
            Err(crate::error::Error::SapiWriteUnavailable)
        }
    }
}

/// Writes binary data to PHP's output stream with output buffering support.
///
/// This function is binary-safe (can handle NUL bytes) AND respects PHP's
/// output buffering (`ob_start()`). Use this when you need both binary-safe
/// output and output buffering compatibility.
///
/// # Arguments
///
/// * `data` - The binary data to write.
///
/// # Returns
///
/// The number of bytes written.
///
/// # Comparison
///
/// | Function | Binary-safe | Output Buffering |
/// |----------|-------------|------------------|
/// | [`printf`] | No | Yes |
/// | [`write()`] | Yes | No (unbuffered) |
/// | [`output_write`] | Yes | Yes |
///
/// # Example
///
/// ```ignore
/// use ext_php_rs::zend::output_write;
///
/// // Binary data that will be captured by ob_start()
/// let data = b"Hello\x00World";
/// output_write(data);
/// ```
#[inline]
#[must_use]
pub fn output_write(data: &[u8]) -> usize {
    unsafe { php_output_write(data.as_ptr().cast::<c_char>(), data.len()) }
}

/// Get the name of the SAPI module.
///
/// # Panics
///
/// * If the module name is not a valid [`CStr`]
///
/// [`CStr`]: std::ffi::CStr
pub fn php_sapi_name() -> String {
    let c_str = unsafe { std::ffi::CStr::from_ptr(sapi_module.name) };
    c_str.to_str().expect("Unable to parse CStr").to_string()
}
