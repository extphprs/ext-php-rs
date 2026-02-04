//! Zend Observer API bindings for PHP error observation.
//!
//! Enables creation of error loggers, APM integrations, and monitoring tools.
//!
//! # Example
//!
//! ```ignore
//! use ext_php_rs::prelude::*;
//!
//! struct MyErrorLogger;
//!
//! impl ErrorObserver for MyErrorLogger {
//!     fn should_observe(&self, error_type: ErrorType) -> bool {
//!         ErrorType::FATAL.contains(error_type)
//!     }
//!
//!     fn on_error(&self, error: &ErrorInfo) {
//!         eprintln!("[{}:{}] {}",
//!             error.filename.unwrap_or("<unknown>"),
//!             error.lineno,
//!             error.message
//!         );
//!     }
//! }
//! ```

use std::sync::OnceLock;

use bitflags::bitflags;

use crate::ffi;

bitflags! {
    /// PHP error types as bitflags for filtering.
    ///
    /// These map directly to PHP's E_* constants and can be combined
    /// for filtering in [`ErrorObserver::should_observe`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Observe only fatal errors
    /// ErrorType::FATAL.contains(error_type)
    ///
    /// // Observe errors and warnings
    /// (ErrorType::FATAL | ErrorType::WARNING).contains(error_type)
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ErrorType: i32 {
        /// Fatal run-time errors (E_ERROR)
        const ERROR             = 1 << 0;
        /// Run-time warnings (E_WARNING)
        const WARNING           = 1 << 1;
        /// Compile-time parse errors (E_PARSE)
        const PARSE             = 1 << 2;
        /// Run-time notices (E_NOTICE)
        const NOTICE            = 1 << 3;
        /// Fatal errors during PHP startup (E_CORE_ERROR)
        const CORE_ERROR        = 1 << 4;
        /// Warnings during PHP startup (E_CORE_WARNING)
        const CORE_WARNING      = 1 << 5;
        /// Fatal compile-time errors (E_COMPILE_ERROR)
        const COMPILE_ERROR     = 1 << 6;
        /// Compile-time warnings (E_COMPILE_WARNING)
        const COMPILE_WARNING   = 1 << 7;
        /// User-generated error (E_USER_ERROR)
        const USER_ERROR        = 1 << 8;
        /// User-generated warning (E_USER_WARNING)
        const USER_WARNING      = 1 << 9;
        /// User-generated notice (E_USER_NOTICE)
        const USER_NOTICE       = 1 << 10;
        /// Strict standards suggestions (E_STRICT)
        #[deprecated = "E_STRICT removed in PHP 8.4, will be removed in PHP 9.0"]
        const STRICT            = 1 << 11;
        /// Catchable fatal error (E_RECOVERABLE_ERROR)
        const RECOVERABLE_ERROR = 1 << 12;
        /// Run-time deprecation notices (E_DEPRECATED)
        const DEPRECATED        = 1 << 13;
        /// User-generated deprecation (E_USER_DEPRECATED)
        const USER_DEPRECATED   = 1 << 14;

        /// All error types (E_ALL, excluding E_STRICT in PHP 8.4+)
        const ALL = Self::ERROR.bits() | Self::WARNING.bits() | Self::PARSE.bits()
                  | Self::NOTICE.bits() | Self::CORE_ERROR.bits() | Self::CORE_WARNING.bits()
                  | Self::COMPILE_ERROR.bits() | Self::COMPILE_WARNING.bits()
                  | Self::USER_ERROR.bits() | Self::USER_WARNING.bits() | Self::USER_NOTICE.bits()
                  | Self::RECOVERABLE_ERROR.bits() | Self::DEPRECATED.bits() | Self::USER_DEPRECATED.bits();

        /// Core errors and warnings (E_CORE)
        const CORE = Self::CORE_ERROR.bits() | Self::CORE_WARNING.bits();

        /// All fatal error types (E_FATAL_ERRORS)
        const FATAL = Self::ERROR.bits() | Self::CORE_ERROR.bits()
                    | Self::COMPILE_ERROR.bits() | Self::USER_ERROR.bits()
                    | Self::RECOVERABLE_ERROR.bits() | Self::PARSE.bits();
    }
}

/// A single frame in a PHP backtrace.
#[derive(Debug, Clone)]
pub struct BacktraceFrame {
    /// Function name (`None` for main script).
    pub function: Option<String>,
    /// Class name for method calls.
    pub class: Option<String>,
    /// Source file.
    pub file: Option<String>,
    /// Line number.
    pub line: u32,
}

impl BacktraceFrame {
    /// # Safety
    ///
    /// `execute_data` must be a valid pointer.
    pub(crate) unsafe fn from_execute_data(
        execute_data: *const ffi::zend_execute_data,
    ) -> Option<Self> {
        if execute_data.is_null() {
            return None;
        }

        let ex = unsafe { &*execute_data };
        let func = ex.func;
        if func.is_null() {
            return Some(Self {
                function: None,
                class: None,
                file: None,
                line: 0,
            });
        }

        let common = unsafe { &(*func).common };

        let function = if common.function_name.is_null() {
            None
        } else {
            unsafe { zend_string_to_string(common.function_name) }
        };

        let class = if common.scope.is_null() {
            None
        } else {
            let ce = unsafe { &*common.scope };
            if ce.name.is_null() {
                None
            } else {
                unsafe { zend_string_to_string(ce.name) }
            }
        };

        #[allow(clippy::cast_possible_truncation)]
        let is_internal = common.type_ == ffi::ZEND_INTERNAL_FUNCTION as u8;

        let (file, line) = if is_internal {
            (None, ex.opline as u32)
        } else {
            let op_array = unsafe { &(*func).op_array };
            let file = if op_array.filename.is_null() {
                None
            } else {
                unsafe { zend_string_to_string(op_array.filename) }
            };
            // Use current opline if available, otherwise use function start line
            let line = if ex.opline.is_null() {
                op_array.line_start
            } else {
                unsafe { (*ex.opline).lineno }
            };
            (file, line)
        };

        Some(Self {
            function,
            class,
            file,
            line,
        })
    }
}

/// Information about a PHP error passed to observers.
#[derive(Debug)]
pub struct ErrorInfo<'a> {
    /// The error type/severity level.
    pub error_type: ErrorType,
    /// Source filename where the error occurred.
    pub filename: Option<&'a str>,
    /// Line number where the error occurred.
    pub lineno: u32,
    /// The error message.
    pub message: &'a str,
}

impl ErrorInfo<'_> {
    /// Captures the current PHP call stack.
    ///
    /// This is lazy - only captures when called, zero cost if unused.
    /// Returns `None` if no execution context is available.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn on_error(&self, error: &ErrorInfo) {
    ///     if let Some(trace) = error.backtrace() {
    ///         for frame in trace {
    ///             eprintln!("  at {}:{}",
    ///                 frame.file.as_deref().unwrap_or("<internal>"),
    ///                 frame.line
    ///             );
    ///         }
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn backtrace(&self) -> Option<Vec<BacktraceFrame>> {
        let eg = unsafe { crate::ffi::ext_php_rs_executor_globals().as_ref()? };
        let mut execute_data = eg.current_execute_data;

        let mut frames = Vec::new();
        while !execute_data.is_null() {
            if let Some(frame) = unsafe { BacktraceFrame::from_execute_data(execute_data) } {
                frames.push(frame);
            }
            execute_data = unsafe { (*execute_data).prev_execute_data };
        }

        if frames.is_empty() {
            None
        } else {
            Some(frames)
        }
    }
}

/// Trait for observing PHP errors.
///
/// # Lifecycle
///
/// 1. `should_observe` is called for each error to filter by type
/// 2. If `true`, `on_error` is called with error details
///
/// # Thread Safety
///
/// Observer must be `Send + Sync`. Use thread-safe primitives for mutable state.
///
/// # Example
///
/// ```ignore
/// use ext_php_rs::prelude::*;
/// use std::sync::atomic::{AtomicU64, Ordering};
///
/// struct ErrorCounter {
///     count: AtomicU64,
/// }
///
/// impl ErrorObserver for ErrorCounter {
///     fn should_observe(&self, error_type: ErrorType) -> bool {
///         ErrorType::FATAL.contains(error_type)
///     }
///
///     fn on_error(&self, error: &ErrorInfo) {
///         self.count.fetch_add(1, Ordering::Relaxed);
///     }
/// }
/// ```
pub trait ErrorObserver: 'static {
    /// Filter which error types to observe.
    ///
    /// Called for every error. Return `true` to receive `on_error` callback.
    /// Use bitflags for efficient filtering.
    fn should_observe(&self, error_type: ErrorType) -> bool;

    /// Called when an observed error occurs.
    fn on_error(&self, error: &ErrorInfo);
}

type ErrorObserverFactory = Box<dyn Fn() -> Box<dyn ErrorObserver + Send + Sync> + Send + Sync>;

static ERROR_OBSERVER_FACTORY: OnceLock<ErrorObserverFactory> = OnceLock::new();
static ERROR_OBSERVER_INSTANCE: OnceLock<Box<dyn ErrorObserver + Send + Sync>> = OnceLock::new();

fn get_error_observer() -> Option<&'static (dyn ErrorObserver + Send + Sync)> {
    ERROR_OBSERVER_INSTANCE
        .get()
        .map(std::convert::AsRef::as_ref)
}

/// # Safety
///
/// Pointer must be valid and string must be valid UTF-8.
unsafe fn zend_string_to_str(zs: *mut ffi::zend_string) -> Option<&'static str> {
    if zs.is_null() {
        return None;
    }
    let len = unsafe { (*zs).len };
    let ptr = unsafe { (*zs).val.as_ptr() };
    let slice = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    std::str::from_utf8(slice).ok()
}

/// # Safety
///
/// Pointer must be valid and string must be valid UTF-8.
unsafe fn zend_string_to_string(zs: *mut ffi::zend_string) -> Option<String> {
    unsafe { zend_string_to_str(zs) }.map(String::from)
}

/// # Safety
///
/// Called from PHP's C code. In ZTS mode, called from the correct thread context.
unsafe extern "C" fn error_observer_callback(
    error_type: std::ffi::c_int,
    error_filename: *mut ffi::zend_string,
    error_lineno: u32,
    message: *mut ffi::zend_string,
) {
    let Some(observer) = get_error_observer() else {
        return;
    };

    let error_type = ErrorType::from_bits_truncate(error_type);

    if !observer.should_observe(error_type) {
        return;
    }

    let filename = unsafe { zend_string_to_str(error_filename) };
    let message_str = unsafe { zend_string_to_str(message) }.unwrap_or("");

    let info = ErrorInfo {
        error_type,
        filename,
        lineno: error_lineno,
        message: message_str,
    };

    observer.on_error(&info);
}

/// # Panics
///
/// Panics if called more than once.
pub(crate) fn register_error_observer_factory(factory: ErrorObserverFactory) {
    assert!(
        ERROR_OBSERVER_FACTORY.set(factory).is_ok(),
        "error_observer can only be registered once per extension"
    );
}

/// # Safety
///
/// Must be called during MINIT phase only.
pub(crate) unsafe fn error_observer_startup() {
    if let Some(factory) = ERROR_OBSERVER_FACTORY.get() {
        if ERROR_OBSERVER_INSTANCE.set(factory()).is_err() {
            return;
        }
        unsafe { ffi::zend_observer_error_register(Some(error_observer_callback)) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestErrorObserver {
        observe_all: bool,
    }

    unsafe impl Send for TestErrorObserver {}
    unsafe impl Sync for TestErrorObserver {}

    impl ErrorObserver for TestErrorObserver {
        fn should_observe(&self, _error_type: ErrorType) -> bool {
            self.observe_all
        }

        fn on_error(&self, _error: &ErrorInfo) {}
    }

    #[test]
    fn test_error_type_bitflags() {
        assert!(ErrorType::FATAL.contains(ErrorType::ERROR));
        assert!(ErrorType::FATAL.contains(ErrorType::PARSE));
        assert!(!ErrorType::FATAL.contains(ErrorType::WARNING));
        assert!(!ErrorType::FATAL.contains(ErrorType::NOTICE));
    }

    #[test]
    fn test_error_type_all() {
        assert!(ErrorType::ALL.contains(ErrorType::ERROR));
        assert!(ErrorType::ALL.contains(ErrorType::WARNING));
        assert!(ErrorType::ALL.contains(ErrorType::NOTICE));
        assert!(ErrorType::ALL.contains(ErrorType::DEPRECATED));
    }

    #[test]
    fn test_observer_trait_impl() {
        let observer = TestErrorObserver { observe_all: true };
        assert!(observer.should_observe(ErrorType::ERROR));

        let observer = TestErrorObserver { observe_all: false };
        assert!(!observer.should_observe(ErrorType::ERROR));
    }

    #[test]
    fn test_error_type_from_bits() {
        let error_type = ErrorType::from_bits_truncate(1); // E_ERROR
        assert_eq!(error_type, ErrorType::ERROR);

        let error_type = ErrorType::from_bits_truncate(2); // E_WARNING
        assert_eq!(error_type, ErrorType::WARNING);

        let error_type = ErrorType::from_bits_truncate(3); // E_ERROR | E_WARNING
        assert!(error_type.contains(ErrorType::ERROR));
        assert!(error_type.contains(ErrorType::WARNING));
    }
}
