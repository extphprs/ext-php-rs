//! Provides implementations for running php code from rust.
//! It only works on linux for now and you should have `php-embed` installed
//!
//! This crate was only test with PHP 8.2 please report any issue with other
//! version You should only use this crate for test purpose, it's not production
//! ready

mod ffi;
mod sapi;

use crate::boxed::ZBox;
use crate::ffi::{
    _zend_file_handle__bindgen_ty_1, ZEND_RESULT_CODE_SUCCESS, php_execute_script,
    zend_destroy_file_handle, zend_eval_string, zend_file_handle, zend_stream_init_filename,
};
use crate::types::{ZendObject, Zval};
use crate::zend::{ExecutorGlobals, panic_wrapper, try_catch};
use parking_lot::{RwLock, const_rwlock};
use std::ffi::{CString, NulError, c_char, c_void};
use std::panic::{AssertUnwindSafe, UnwindSafe, resume_unwind};
use std::path::Path;
use std::ptr::null_mut;

pub use ffi::*;
pub use sapi::SapiModule;

/// The embed module provides a way to run php code from rust
pub struct Embed;

/// Error type for the embed module
#[derive(Debug)]
pub enum EmbedError {
    /// Failed to initialize
    InitError,
    /// The script exited with a non-zero code
    ExecuteError(Option<ZBox<ZendObject>>),
    /// The script exited with a non-zero code
    ExecuteScriptError,
    /// The script is not a valid [`CString`]
    InvalidEvalString(NulError),
    /// Failed to open the script file at the given path
    InvalidPath,
    /// The script was executed but an exception was thrown
    CatchError,
}

impl EmbedError {
    /// Check if the error is a bailout
    #[must_use]
    pub fn is_bailout(&self) -> bool {
        matches!(self, EmbedError::CatchError)
    }
}

static RUN_FN_LOCK: RwLock<()> = const_rwlock(());

impl Embed {
    /// Run a php script from a file
    ///
    /// This function will only work correctly when used inside the `Embed::run`
    /// function otherwise behavior is unexpected
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The script was executed successfully
    ///
    /// # Errors
    ///
    /// * `Err(EmbedError)` - An error occurred during the execution of the
    ///   script
    ///
    /// # Example
    ///
    /// ```
    /// use ext_php_rs::embed::Embed;
    ///
    /// Embed::run(|| {
    ///     let result = Embed::run_script("src/embed/test-script.php");
    ///
    ///     assert!(result.is_ok());
    /// });
    /// ```
    pub fn run_script<P: AsRef<Path>>(path: P) -> Result<(), EmbedError> {
        let path = match path.as_ref().to_str() {
            Some(path) => match CString::new(path) {
                Ok(path) => path,
                Err(err) => return Err(EmbedError::InvalidEvalString(err)),
            },
            None => return Err(EmbedError::InvalidPath),
        };

        let mut file_handle = zend_file_handle {
            #[allow(clippy::used_underscore_items)]
            handle: _zend_file_handle__bindgen_ty_1 { fp: null_mut() },
            filename: null_mut(),
            opened_path: null_mut(),
            type_: 0,
            primary_script: false,
            in_list: false,
            buf: null_mut(),
            len: 0,
        };

        unsafe {
            zend_stream_init_filename(&raw mut file_handle, path.as_ptr());
        }

        let exec_result = try_catch(AssertUnwindSafe(|| unsafe {
            php_execute_script(&raw mut file_handle)
        }));

        unsafe { zend_destroy_file_handle(&raw mut file_handle) }

        match exec_result {
            Err(_) => Err(EmbedError::CatchError),
            Ok(true) => Ok(()),
            Ok(false) => Err(EmbedError::ExecuteScriptError),
        }
    }

    /// Start and run embed sapi engine
    ///
    /// This function will allow to run php code from rust, the same PHP context
    /// is keep between calls inside the function passed to this method.
    /// Which means subsequent calls to `Embed::eval` or `Embed::run_script`
    /// will be able to access variables defined in previous calls
    ///
    /// # Returns
    ///
    /// * R - The result of the function passed to this method
    ///
    /// R must implement [`Default`] so it can be returned in case of a bailout
    ///
    /// # Example
    ///
    /// ```
    /// use ext_php_rs::embed::Embed;
    ///
    /// Embed::run(|| {
    ///    let _ = Embed::eval("$foo = 'foo';");
    ///    let foo = Embed::eval("$foo;");
    ///    assert!(foo.is_ok());
    ///    assert_eq!(foo.unwrap().string().unwrap(), "foo");
    /// });
    /// ```
    pub fn run<R, F: FnOnce() -> R + UnwindSafe>(func: F) -> R
    where
        R: Default,
    {
        // @TODO handle php thread safe
        //
        // This is to prevent multiple threads from running php at the same time
        // At some point we should detect if php is compiled with thread safety and
        // avoid doing that in this case
        let _guard = RUN_FN_LOCK.write();

        let panic = unsafe {
            ext_php_rs_embed_callback(
                0,
                null_mut(),
                panic_wrapper::<R, F>,
                (&raw const func).cast::<c_void>(),
            )
        };

        // Prevent the closure from being dropped here since it was consumed in panic_wrapper
        std::mem::forget(func);

        // This can happen if there is a bailout
        if panic.is_null() {
            return R::default();
        }

        match unsafe { *Box::from_raw(panic.cast::<std::thread::Result<R>>()) } {
            Ok(r) => r,
            Err(err) => {
                // we resume the panic here so it can be caught correctly by the test framework
                resume_unwind(err);
            }
        }
    }

    /// Evaluate a php code
    ///
    /// This function will only work correctly when used inside the `Embed::run`
    /// function
    ///
    /// # Returns
    ///
    /// * `Ok(Zval)` - The result of the evaluation
    ///
    /// # Errors
    ///
    /// * `Err(EmbedError)` - An error occurred during the evaluation
    ///
    /// # Example
    ///
    /// ```
    /// use ext_php_rs::embed::Embed;
    ///
    /// Embed::run(|| {
    ///    let foo = Embed::eval("$foo = 'foo';");
    ///    assert!(foo.is_ok());
    /// });
    /// ```
    pub fn eval(code: &str) -> Result<Zval, EmbedError> {
        let cstr = match CString::new(code) {
            Ok(cstr) => cstr,
            Err(err) => return Err(EmbedError::InvalidEvalString(err)),
        };

        let mut result = Zval::new();

        let exec_result = try_catch(AssertUnwindSafe(|| unsafe {
            zend_eval_string(
                cstr.as_ptr().cast::<c_char>(),
                &raw mut result,
                c"run".as_ptr().cast(),
            )
        }));

        match exec_result {
            Err(_) => Err(EmbedError::CatchError),
            Ok(ZEND_RESULT_CODE_SUCCESS) => Ok(result),
            Ok(_) => Err(EmbedError::ExecuteError(ExecutorGlobals::take_exception())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::Embed;

    #[test]
    fn test_run() {
        Embed::run(|| {
            let result = Embed::eval("$foo = 'foo';");

            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_run_error() {
        Embed::run(|| {
            let result = Embed::eval("stupid code;");

            assert!(result.is_err());
        });
    }

    #[test]
    fn test_run_script() {
        Embed::run(|| {
            let result = Embed::run_script("src/embed/test-script.php");

            assert!(result.is_ok());

            let zval = Embed::eval("$foo;").unwrap();

            assert!(zval.is_object());

            let obj = zval.object().unwrap();

            assert_eq!(obj.get_class_name().unwrap(), "Test");
        });
    }

    #[test]
    fn test_run_script_error() {
        Embed::run(|| {
            let result = Embed::run_script("src/embed/test-script-exception.php");

            assert!(result.is_err());
        });
    }

    #[test]
    #[should_panic(expected = "test panic")]
    fn test_panic() {
        Embed::run::<(), _>(|| {
            panic!("test panic");
        });
    }

    #[test]
    fn test_return() {
        let foo = Embed::run(|| "foo");

        assert_eq!(foo, "foo");
    }

    #[test]
    fn test_eval_bailout() {
        Embed::run(|| {
            // TODO: For PHP 8.5, this needs to be replaced, as `E_USER_ERROR` is deprecated.
            //       Currently, this seems to still be the best way to trigger a bailout.
            let result = Embed::eval("trigger_error(\"Fatal error\", E_USER_ERROR);");

            assert!(result.is_err());
            assert!(result.unwrap_err().is_bailout());
        });
    }

    #[test]
    fn test_php_write() {
        use crate::zend::write;

        Embed::run(|| {
            // Test write function with regular data
            let bytes_written = write(b"Hello").expect("write failed");
            assert_eq!(bytes_written, 5);

            // Test write function with binary data containing NUL bytes
            let bytes_written = write(b"Hello\x00World").expect("write failed");
            assert_eq!(bytes_written, 11);

            // Test php_write! macro with byte literal
            let bytes_written = php_write!(b"Test").expect("php_write failed");
            assert_eq!(bytes_written, 4);

            // Test php_write! macro with binary data containing NUL bytes
            let bytes_written = php_write!(b"Binary\x00Data\x00Here").expect("php_write failed");
            assert_eq!(bytes_written, 16);

            // Test php_write! macro with byte slice variable
            let data: &[u8] = &[0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
            let bytes_written = php_write!(data).expect("php_write failed");
            assert_eq!(bytes_written, 5);

            // Test empty data
            let bytes_written = write(b"").expect("write failed");
            assert_eq!(bytes_written, 0);
        });
    }

    #[test]
    fn test_php_write_bypasses_output_buffering() {
        use crate::zend::write;

        Embed::run(|| {
            // Start PHP output buffering
            Embed::eval("ob_start();").expect("ob_start failed");

            // Write data using ub_write - this bypasses output buffering
            // ("ub" = unbuffered) and goes directly to SAPI output
            write(b"Direct output").expect("write failed");

            // Get the buffered output - should be empty since ub_write bypasses buffering
            let result = Embed::eval("ob_get_clean();").expect("ob_get_clean failed");
            let output = result.string().expect("expected string result");

            // Verify that ub_write bypasses output buffering
            assert_eq!(output, "", "ub_write should bypass output buffering");
        });
    }

    #[test]
    fn test_php_print_respects_output_buffering() {
        use crate::zend::printf;

        Embed::run(|| {
            // Start PHP output buffering
            Embed::eval("ob_start();").expect("ob_start failed");

            // Write data using php_printf - this goes through output buffering
            printf("Hello from Rust").expect("printf failed");

            // Get the buffered output
            let result = Embed::eval("ob_get_clean();").expect("ob_get_clean failed");
            let output = result.string().expect("expected string result");

            // Verify that printf output is captured by output buffering
            assert_eq!(output, "Hello from Rust");
        });
    }

    #[test]
    fn test_php_output_write_binary_safe_with_buffering() {
        use crate::zend::output_write;

        Embed::run(|| {
            // Start PHP output buffering
            Embed::eval("ob_start();").expect("ob_start failed");

            // Write binary data with NUL bytes - should be captured by buffer
            let bytes_written = output_write(b"Hello\x00World");
            assert_eq!(bytes_written, 11);

            // Get the buffered output
            let result = Embed::eval("ob_get_clean();").expect("ob_get_clean failed");
            let output = result.string().expect("expected string result");

            // Verify binary data was captured correctly (including NUL byte)
            assert_eq!(output, "Hello\x00World");
        });
    }
}
