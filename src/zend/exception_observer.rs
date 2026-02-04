//! Zend Exception Hook bindings for PHP exception observation.
//!
//! Enables creation of exception loggers, APM integrations, and monitoring tools.
//!
//! # Example
//!
//! ```ignore
//! use ext_php_rs::prelude::*;
//!
//! struct MyExceptionLogger;
//!
//! impl ExceptionObserver for MyExceptionLogger {
//!     fn on_exception(&self, exception: &ExceptionInfo) {
//!         eprintln!("[EXCEPTION] {}: {}",
//!             exception.class_name,
//!             exception.message.as_deref().unwrap_or("<no message>")
//!         );
//!     }
//! }
//! ```

use std::sync::OnceLock;

use crate::ffi;

/// Information about a PHP exception passed to observers.
#[derive(Debug)]
pub struct ExceptionInfo {
    /// The exception class name (e.g., "Exception", "`RuntimeException`").
    pub class_name: String,
    /// The exception message.
    pub message: Option<String>,
    /// The exception code.
    pub code: i64,
    /// Source filename where the exception was thrown.
    pub file: Option<String>,
    /// Line number where the exception was thrown.
    pub line: u32,
}

impl ExceptionInfo {
    /// # Safety
    ///
    /// `exception` must be a valid pointer to a `zend_object` representing a Throwable.
    unsafe fn from_zend_object(exception: *mut ffi::zend_object) -> Option<Self> {
        if exception.is_null() {
            return None;
        }

        let ex = unsafe { &*exception };

        let class_name = if ex.ce.is_null() {
            "Unknown".to_string()
        } else {
            let ce = unsafe { &*ex.ce };
            if ce.name.is_null() {
                "Unknown".to_string()
            } else {
                unsafe { zend_string_to_string(ce.name) }.unwrap_or_else(|| "Unknown".to_string())
            }
        };

        let message = unsafe { read_exception_property(exception, "message") };
        let code = unsafe { read_exception_property_long(exception, "code") };
        let file = unsafe { read_exception_property(exception, "file") };
        let line = unsafe { read_exception_property_long(exception, "line") }
            .try_into()
            .unwrap_or(0);

        Some(Self {
            class_name,
            message,
            code,
            file,
            line,
        })
    }

    /// Captures the current PHP call stack at exception throw time.
    ///
    /// This is lazy - only captures when called, zero cost if unused.
    /// Returns `None` if no execution context is available.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn on_exception(&self, exception: &ExceptionInfo) {
    ///     if let Some(trace) = exception.backtrace() {
    ///         for frame in trace {
    ///             eprintln!("  at {}:{} in {}",
    ///                 frame.file.as_deref().unwrap_or("<internal>"),
    ///                 frame.line,
    ///                 frame.function.as_deref().unwrap_or("<main>")
    ///             );
    ///         }
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn backtrace(&self) -> Option<Vec<super::BacktraceFrame>> {
        let eg = unsafe { crate::ffi::ext_php_rs_executor_globals().as_ref()? };
        let mut execute_data = eg.current_execute_data;

        let mut frames = Vec::new();
        while !execute_data.is_null() {
            if let Some(frame) = unsafe { super::BacktraceFrame::from_execute_data(execute_data) } {
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

/// # Safety
///
/// `exception` must be a valid pointer to a `zend_object`.
unsafe fn read_exception_property(exception: *mut ffi::zend_object, name: &str) -> Option<String> {
    let ce = unsafe { (*exception).ce };
    if ce.is_null() {
        return None;
    }

    let name_cstr = std::ffi::CString::new(name).ok()?;
    let mut rv = std::mem::MaybeUninit::<ffi::zval>::uninit();

    let prop = unsafe {
        ffi::zend_read_property(
            ce,
            exception,
            name_cstr.as_ptr(),
            name.len(),
            true,
            rv.as_mut_ptr(),
        )
    };

    if prop.is_null() {
        return None;
    }

    let prop_ref = unsafe { &*prop };
    let type_info = unsafe { prop_ref.u1.type_info };

    if (type_info & 0xFF) as u32 == ffi::IS_STRING {
        let zs = unsafe { prop_ref.value.str_ };
        unsafe { zend_string_to_string(zs) }
    } else {
        None
    }
}

/// # Safety
///
/// `exception` must be a valid pointer to a `zend_object`.
unsafe fn read_exception_property_long(exception: *mut ffi::zend_object, name: &str) -> i64 {
    let ce = unsafe { (*exception).ce };
    if ce.is_null() {
        return 0;
    }

    let Ok(name_cstr) = std::ffi::CString::new(name) else {
        return 0;
    };
    let mut rv = std::mem::MaybeUninit::<ffi::zval>::uninit();

    let prop = unsafe {
        ffi::zend_read_property(
            ce,
            exception,
            name_cstr.as_ptr(),
            name.len(),
            true,
            rv.as_mut_ptr(),
        )
    };

    if prop.is_null() {
        return 0;
    }

    let prop_ref = unsafe { &*prop };
    let type_info = unsafe { prop_ref.u1.type_info };

    if (type_info & 0xFF) as u32 == ffi::IS_LONG {
        unsafe { prop_ref.value.lval }
    } else {
        0
    }
}

/// # Safety
///
/// Pointer must be valid and string must be valid UTF-8.
unsafe fn zend_string_to_string(zs: *mut ffi::zend_string) -> Option<String> {
    if zs.is_null() {
        return None;
    }
    let len = unsafe { (*zs).len };
    let ptr = unsafe { (*zs).val.as_ptr() };
    let slice = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    std::str::from_utf8(slice).ok().map(String::from)
}

/// Trait for observing PHP exceptions.
///
/// # Lifecycle
///
/// `on_exception` is called whenever an exception is thrown in PHP,
/// before any catch blocks are evaluated.
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
/// struct ExceptionCounter {
///     count: AtomicU64,
/// }
///
/// impl ExceptionObserver for ExceptionCounter {
///     fn on_exception(&self, exception: &ExceptionInfo) {
///         self.count.fetch_add(1, Ordering::Relaxed);
///         eprintln!("Exception #{}: {}",
///             self.count.load(Ordering::Relaxed),
///             exception.class_name
///         );
///     }
/// }
/// ```
pub trait ExceptionObserver: 'static {
    /// Called when an exception is thrown.
    ///
    /// This is called at throw time, before any catch blocks are evaluated.
    /// The exception may or may not be caught.
    fn on_exception(&self, exception: &ExceptionInfo);
}

type ExceptionObserverFactory =
    Box<dyn Fn() -> Box<dyn ExceptionObserver + Send + Sync> + Send + Sync>;

static EXCEPTION_OBSERVER_FACTORY: OnceLock<ExceptionObserverFactory> = OnceLock::new();
static EXCEPTION_OBSERVER_INSTANCE: OnceLock<Box<dyn ExceptionObserver + Send + Sync>> =
    OnceLock::new();

static PREVIOUS_HOOK: OnceLock<Option<unsafe extern "C" fn(*mut ffi::zend_object)>> =
    OnceLock::new();

fn get_exception_observer() -> Option<&'static (dyn ExceptionObserver + Send + Sync)> {
    EXCEPTION_OBSERVER_INSTANCE
        .get()
        .map(std::convert::AsRef::as_ref)
}

/// # Safety
///
/// Called from PHP's C code when an exception is thrown.
unsafe extern "C" fn exception_observer_callback(exception: *mut ffi::zend_object) {
    if let Some(observer) = get_exception_observer()
        && let Some(info) = (unsafe { ExceptionInfo::from_zend_object(exception) })
    {
        observer.on_exception(&info);
    }

    if let Some(Some(prev)) = PREVIOUS_HOOK.get() {
        unsafe { prev(exception) };
    }
}

/// # Panics
///
/// Panics if called more than once.
pub(crate) fn register_exception_observer_factory(factory: ExceptionObserverFactory) {
    assert!(
        EXCEPTION_OBSERVER_FACTORY.set(factory).is_ok(),
        "exception_observer can only be registered once per extension"
    );
}

/// # Safety
///
/// Must be called during MINIT phase only.
pub(crate) unsafe fn exception_observer_startup() {
    if let Some(factory) = EXCEPTION_OBSERVER_FACTORY.get() {
        if EXCEPTION_OBSERVER_INSTANCE.set(factory()).is_err() {
            return;
        }

        let prev = unsafe { ffi::zend_throw_exception_hook };
        let _ = PREVIOUS_HOOK.set(prev);

        unsafe {
            ffi::zend_throw_exception_hook = Some(exception_observer_callback);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExceptionObserver;

    unsafe impl Send for TestExceptionObserver {}
    unsafe impl Sync for TestExceptionObserver {}

    impl ExceptionObserver for TestExceptionObserver {
        fn on_exception(&self, _exception: &ExceptionInfo) {}
    }

    #[test]
    fn test_exception_info_fields() {
        let info = ExceptionInfo {
            class_name: "RuntimeException".to_string(),
            message: Some("Test error".to_string()),
            code: 42,
            file: Some("/path/to/file.php".to_string()),
            line: 100,
        };

        assert_eq!(info.class_name, "RuntimeException");
        assert_eq!(info.message, Some("Test error".to_string()));
        assert_eq!(info.code, 42);
        assert_eq!(info.file, Some("/path/to/file.php".to_string()));
        assert_eq!(info.line, 100);
    }

    #[test]
    fn test_observer_trait_impl() {
        let observer = TestExceptionObserver;
        let info = ExceptionInfo {
            class_name: "Exception".to_string(),
            message: None,
            code: 0,
            file: None,
            line: 0,
        };
        observer.on_exception(&info);
    }
}
