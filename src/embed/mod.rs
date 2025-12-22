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
use std::panic::{RefUnwindSafe, resume_unwind};
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

        let exec_result = try_catch(|| unsafe { php_execute_script(&raw mut file_handle) });

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
    pub fn run<R, F: FnMut() -> R + RefUnwindSafe>(func: F) -> R
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

        let exec_result = try_catch(|| unsafe {
            zend_eval_string(
                cstr.as_ptr().cast::<c_char>(),
                &raw mut result,
                c"run".as_ptr().cast(),
            )
        });

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
            // TODO: For PHP 8.5, this needs to be replaced, as `E_USER_ERROR` is
            // deprecated.       Currently, this seems to still be the best way
            // to trigger a bailout.
            let result = Embed::eval("trigger_error(\"Fatal error\", E_USER_ERROR);");

            assert!(result.is_err());
            assert!(result.unwrap_err().is_bailout());
        });
    }
}
