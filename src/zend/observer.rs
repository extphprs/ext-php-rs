//! Zend Observer API bindings for function call observation.
//!
//! Enables creation of profilers, tracers, and instrumentation tools.
//!
//! # Example
//!
//! ```ignore
//! use ext_php_rs::prelude::*;
//!
//! struct MyProfiler;
//!
//! impl FcallObserver for MyProfiler {
//!     fn should_observe(&self, info: &FcallInfo) -> bool {
//!         !info.is_internal
//!     }
//!
//!     fn begin(&self, _execute_data: &ExecuteData) {}
//!     fn end(&self, _execute_data: &ExecuteData, _retval: Option<&Zval>) {}
//! }
//! ```

use std::sync::OnceLock;

use crate::ffi;
use crate::types::Zval;
use crate::zend::ExecuteData;

/// Metadata about a function being called.
#[derive(Debug, Clone)]
pub struct FcallInfo<'a> {
    /// Function name (`None` for anonymous functions or main script).
    pub function_name: Option<&'a str>,
    /// Class name for method calls.
    pub class_name: Option<&'a str>,
    /// Source filename (`None` for internal functions).
    pub filename: Option<&'a str>,
    /// Line number (0 for internal functions).
    pub lineno: u32,
    /// Whether this is an internal (built-in) PHP function.
    pub is_internal: bool,
}

impl FcallInfo<'_> {
    fn empty() -> Self {
        Self {
            function_name: None,
            class_name: None,
            filename: None,
            lineno: 0,
            is_internal: false,
        }
    }
}

/// Trait for observing PHP function calls.
///
/// # Lifecycle
///
/// 1. `should_observe` is called once per unique function (cached by PHP)
/// 2. If `true`, `begin` is called at function entry
/// 3. `end` is called at function exit (even on exceptions)
///
/// # Thread Safety
///
/// Observer must be `Send + Sync`. Use thread-safe primitives for mutable state.
pub trait FcallObserver: 'static {
    /// Whether to observe a specific function (result is cached by PHP).
    fn should_observe(&self, info: &FcallInfo) -> bool;

    /// Called when an observed function begins execution.
    fn begin(&self, execute_data: &ExecuteData);

    /// Called when an observed function ends execution (even on exceptions).
    fn end(&self, execute_data: &ExecuteData, retval: Option<&Zval>);
}

type ObserverFactory = Box<dyn Fn() -> Box<dyn FcallObserver + Send + Sync> + Send + Sync>;

static OBSERVER_FACTORY: OnceLock<ObserverFactory> = OnceLock::new();
static OBSERVER_INSTANCE: OnceLock<Box<dyn FcallObserver + Send + Sync>> = OnceLock::new();

impl FcallInfo<'_> {
    /// # Safety
    ///
    /// `execute_data` must be a valid pointer.
    pub(crate) unsafe fn from_execute_data(
        execute_data: *mut ffi::zend_execute_data,
    ) -> FcallInfo<'static> {
        if execute_data.is_null() {
            return FcallInfo::empty();
        }

        let func = unsafe { (*execute_data).func };
        if func.is_null() {
            return FcallInfo::empty();
        }

        let common = unsafe { &(*func).common };
        let func_type = common.type_;
        #[allow(clippy::cast_possible_truncation)]
        let is_internal = func_type == ffi::ZEND_INTERNAL_FUNCTION as u8;

        let function_name = if common.function_name.is_null() {
            None
        } else {
            unsafe { zend_string_to_str(common.function_name) }
        };

        let class_name = if common.scope.is_null() {
            None
        } else {
            let ce = unsafe { &*common.scope };
            if ce.name.is_null() {
                None
            } else {
                unsafe { zend_string_to_str(ce.name) }
            }
        };

        let (filename, lineno) = if is_internal {
            (None, 0)
        } else {
            let op_array = unsafe { &(*func).op_array };
            let filename = if op_array.filename.is_null() {
                None
            } else {
                unsafe { zend_string_to_str(op_array.filename) }
            };
            (filename, op_array.line_start)
        };

        FcallInfo {
            function_name,
            class_name,
            filename,
            lineno,
            is_internal,
        }
    }
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

fn get_observer() -> Option<&'static (dyn FcallObserver + Send + Sync)> {
    OBSERVER_INSTANCE.get().map(std::convert::AsRef::as_ref)
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn observer_fcall_init(
    execute_data: *mut ffi::zend_execute_data,
) -> ffi::zend_observer_fcall_handlers {
    let empty = ffi::zend_observer_fcall_handlers {
        begin: None,
        end: None,
    };

    let Some(observer) = get_observer() else {
        return empty;
    };

    let info = unsafe { FcallInfo::from_execute_data(execute_data) };

    if observer.should_observe(&info) {
        ffi::zend_observer_fcall_handlers {
            begin: Some(observer_begin),
            end: Some(observer_end),
        }
    } else {
        empty
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn observer_begin(execute_data: *mut ffi::zend_execute_data) {
    if let Some(observer) = get_observer()
        && let Some(ex) = unsafe { execute_data.as_ref() }
    {
        observer.begin(ex);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn observer_end(
    execute_data: *mut ffi::zend_execute_data,
    retval: *mut ffi::zval,
) {
    if let Some(observer) = get_observer()
        && let Some(ex) = unsafe { execute_data.as_ref() }
    {
        let retval_ref = if retval.is_null() {
            None
        } else {
            Some(unsafe { &*retval.cast_const().cast::<Zval>() })
        };
        observer.end(ex, retval_ref);
    }
}

/// # Panics
///
/// Panics if called more than once.
pub(crate) fn register_fcall_observer_factory(factory: ObserverFactory) {
    assert!(
        OBSERVER_FACTORY.set(factory).is_ok(),
        "fcall_observer can only be registered once per extension"
    );
}

/// # Safety
///
/// Must be called during MINIT phase only.
pub(crate) unsafe fn observer_startup() {
    if let Some(factory) = OBSERVER_FACTORY.get() {
        if OBSERVER_INSTANCE.set(factory()).is_err() {
            return;
        }
        unsafe { ffi::zend_observer_fcall_register(Some(observer_fcall_init)) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestObserver {
        observe_all: bool,
    }

    unsafe impl Send for TestObserver {}
    unsafe impl Sync for TestObserver {}

    impl FcallObserver for TestObserver {
        fn should_observe(&self, _info: &FcallInfo) -> bool {
            self.observe_all
        }

        fn begin(&self, _execute_data: &ExecuteData) {}

        fn end(&self, _execute_data: &ExecuteData, _retval: Option<&Zval>) {}
    }

    #[test]
    fn test_fcall_info_empty() {
        let info = FcallInfo::empty();
        assert!(info.function_name.is_none());
        assert!(info.class_name.is_none());
        assert!(info.filename.is_none());
        assert_eq!(info.lineno, 0);
        assert!(!info.is_internal);
    }

    #[test]
    fn test_observer_trait_impl() {
        let observer = TestObserver { observe_all: true };
        let info = FcallInfo::empty();
        assert!(observer.should_observe(&info));

        let observer = TestObserver { observe_all: false };
        assert!(!observer.should_observe(&info));
    }
}
