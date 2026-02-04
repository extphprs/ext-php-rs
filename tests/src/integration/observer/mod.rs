//! Integration tests for the Observer API (fcall, error, and exception observers).

use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;
use std::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Function Call Observer Tests
// ============================================================================

/// Test observer that counts user function calls.
pub struct TestObserver {
    call_count: AtomicU64,
    end_count: AtomicU64,
}

impl TestObserver {
    fn new() -> Self {
        Self {
            call_count: AtomicU64::new(0),
            end_count: AtomicU64::new(0),
        }
    }

    fn get_call_count(&self) -> u64 {
        self.call_count.load(Ordering::Relaxed)
    }

    fn get_end_count(&self) -> u64 {
        self.end_count.load(Ordering::Relaxed)
    }
}

impl FcallObserver for TestObserver {
    fn should_observe(&self, info: &FcallInfo) -> bool {
        // Only observe user-defined functions, not internal PHP functions
        !info.is_internal
    }

    fn begin(&self, _execute_data: &ExecuteData) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
    }

    fn end(&self, _execute_data: &ExecuteData, _retval: Option<&Zval>) {
        self.end_count.fetch_add(1, Ordering::Relaxed);
    }
}

// Static observer instance for testing (needed to access counts from PHP functions)
static OBSERVER: std::sync::OnceLock<TestObserver> = std::sync::OnceLock::new();

fn get_or_init_observer() -> &'static TestObserver {
    OBSERVER.get_or_init(TestObserver::new)
}

/// Get the current call count from the observer.
#[php_function]
pub fn observer_test_get_call_count() -> u64 {
    get_or_init_observer().get_call_count()
}

/// Get the current end count from the observer.
#[php_function]
pub fn observer_test_get_end_count() -> u64 {
    get_or_init_observer().get_end_count()
}

/// Reset the observer counters.
#[php_function]
pub fn observer_test_reset() {
    let observer = get_or_init_observer();
    observer.call_count.store(0, Ordering::Relaxed);
    observer.end_count.store(0, Ordering::Relaxed);
}

/// Wrapper observer that delegates to the static instance.
struct TestObserverWrapper;

impl FcallObserver for TestObserverWrapper {
    fn should_observe(&self, info: &FcallInfo) -> bool {
        get_or_init_observer().should_observe(info)
    }

    fn begin(&self, execute_data: &ExecuteData) {
        get_or_init_observer().begin(execute_data);
    }

    fn end(&self, execute_data: &ExecuteData, retval: Option<&Zval>) {
        get_or_init_observer().end(execute_data, retval);
    }
}

// ============================================================================
// Error Observer Tests
// ============================================================================

/// Test error observer that counts errors by type.
pub struct TestErrorObserver {
    error_count: AtomicU64,
    warning_count: AtomicU64,
    last_message: std::sync::RwLock<String>,
    last_file: std::sync::RwLock<String>,
    last_line: AtomicU64,
}

impl TestErrorObserver {
    fn new() -> Self {
        Self {
            error_count: AtomicU64::new(0),
            warning_count: AtomicU64::new(0),
            last_message: std::sync::RwLock::new(String::new()),
            last_file: std::sync::RwLock::new(String::new()),
            last_line: AtomicU64::new(0),
        }
    }

    fn get_error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    fn get_warning_count(&self) -> u64 {
        self.warning_count.load(Ordering::Relaxed)
    }

    fn get_last_message(&self) -> String {
        self.last_message.read().unwrap().clone()
    }

    fn get_last_file(&self) -> String {
        self.last_file.read().unwrap().clone()
    }

    fn get_last_line(&self) -> u64 {
        self.last_line.load(Ordering::Relaxed)
    }

    fn reset(&self) {
        self.error_count.store(0, Ordering::Relaxed);
        self.warning_count.store(0, Ordering::Relaxed);
        *self.last_message.write().unwrap() = String::new();
        *self.last_file.write().unwrap() = String::new();
        self.last_line.store(0, Ordering::Relaxed);
    }
}

impl ErrorObserver for TestErrorObserver {
    fn should_observe(&self, error_type: ErrorType) -> bool {
        // Observe warnings and user errors/warnings
        (ErrorType::WARNING | ErrorType::USER_WARNING | ErrorType::USER_ERROR).contains(error_type)
    }

    fn on_error(&self, error: &ErrorInfo) {
        if ErrorType::WARNING.contains(error.error_type)
            || ErrorType::USER_WARNING.contains(error.error_type)
        {
            self.warning_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }

        *self.last_message.write().unwrap() = error.message.to_string();
        *self.last_file.write().unwrap() = error.filename.unwrap_or("").to_string();
        self.last_line
            .store(u64::from(error.lineno), Ordering::Relaxed);
    }
}

// Static error observer instance for testing
static ERROR_OBSERVER: std::sync::OnceLock<TestErrorObserver> = std::sync::OnceLock::new();

fn get_or_init_error_observer() -> &'static TestErrorObserver {
    ERROR_OBSERVER.get_or_init(TestErrorObserver::new)
}

/// Get the current error count from the error observer.
#[php_function]
pub fn error_observer_test_get_error_count() -> u64 {
    get_or_init_error_observer().get_error_count()
}

/// Get the current warning count from the error observer.
#[php_function]
pub fn error_observer_test_get_warning_count() -> u64 {
    get_or_init_error_observer().get_warning_count()
}

/// Get the last error message.
#[php_function]
pub fn error_observer_test_get_last_message() -> String {
    get_or_init_error_observer().get_last_message()
}

/// Get the last error file.
#[php_function]
pub fn error_observer_test_get_last_file() -> String {
    get_or_init_error_observer().get_last_file()
}

/// Get the last error line number.
#[php_function]
pub fn error_observer_test_get_last_line() -> u64 {
    get_or_init_error_observer().get_last_line()
}

/// Reset the error observer counters.
#[php_function]
pub fn error_observer_test_reset() {
    get_or_init_error_observer().reset();
}

/// Wrapper error observer that delegates to the static instance.
struct TestErrorObserverWrapper;

impl ErrorObserver for TestErrorObserverWrapper {
    fn should_observe(&self, error_type: ErrorType) -> bool {
        get_or_init_error_observer().should_observe(error_type)
    }

    fn on_error(&self, error: &ErrorInfo) {
        get_or_init_error_observer().on_error(error);
    }
}

// ============================================================================
// Exception Observer Tests
// ============================================================================

/// Test exception observer that tracks thrown exceptions.
pub struct TestExceptionObserver {
    exception_count: AtomicU64,
    last_class: std::sync::RwLock<String>,
    last_message: std::sync::RwLock<String>,
    last_file: std::sync::RwLock<String>,
    last_line: AtomicU64,
    last_code: std::sync::atomic::AtomicI64,
    last_backtrace_depth: AtomicU64,
    last_backtrace_functions: std::sync::RwLock<Vec<String>>,
}

impl TestExceptionObserver {
    fn new() -> Self {
        Self {
            exception_count: AtomicU64::new(0),
            last_class: std::sync::RwLock::new(String::new()),
            last_message: std::sync::RwLock::new(String::new()),
            last_file: std::sync::RwLock::new(String::new()),
            last_line: AtomicU64::new(0),
            last_code: std::sync::atomic::AtomicI64::new(0),
            last_backtrace_depth: AtomicU64::new(0),
            last_backtrace_functions: std::sync::RwLock::new(Vec::new()),
        }
    }

    fn get_exception_count(&self) -> u64 {
        self.exception_count.load(Ordering::Relaxed)
    }

    fn get_last_class(&self) -> String {
        self.last_class.read().unwrap().clone()
    }

    fn get_last_message(&self) -> String {
        self.last_message.read().unwrap().clone()
    }

    fn get_last_file(&self) -> String {
        self.last_file.read().unwrap().clone()
    }

    fn get_last_line(&self) -> u64 {
        self.last_line.load(Ordering::Relaxed)
    }

    fn get_last_code(&self) -> i64 {
        self.last_code.load(Ordering::Relaxed)
    }

    fn get_last_backtrace_depth(&self) -> u64 {
        self.last_backtrace_depth.load(Ordering::Relaxed)
    }

    fn get_last_backtrace_functions(&self) -> Vec<String> {
        self.last_backtrace_functions.read().unwrap().clone()
    }

    fn reset(&self) {
        self.exception_count.store(0, Ordering::Relaxed);
        *self.last_class.write().unwrap() = String::new();
        *self.last_message.write().unwrap() = String::new();
        *self.last_file.write().unwrap() = String::new();
        self.last_line.store(0, Ordering::Relaxed);
        self.last_code.store(0, Ordering::Relaxed);
        self.last_backtrace_depth.store(0, Ordering::Relaxed);
        self.last_backtrace_functions.write().unwrap().clear();
    }
}

impl ExceptionObserver for TestExceptionObserver {
    fn on_exception(&self, exception: &ExceptionInfo) {
        self.exception_count.fetch_add(1, Ordering::Relaxed);
        self.last_class
            .write()
            .unwrap()
            .clone_from(&exception.class_name);
        *self.last_message.write().unwrap() = exception.message.clone().unwrap_or_default();
        *self.last_file.write().unwrap() = exception.file.clone().unwrap_or_default();
        self.last_line
            .store(u64::from(exception.line), Ordering::Relaxed);
        self.last_code.store(exception.code, Ordering::Relaxed);

        if let Some(backtrace) = exception.backtrace() {
            self.last_backtrace_depth
                .store(backtrace.len() as u64, Ordering::Relaxed);
            let functions: Vec<String> = backtrace
                .iter()
                .filter_map(|frame| frame.function.clone())
                .collect();
            *self.last_backtrace_functions.write().unwrap() = functions;
        } else {
            self.last_backtrace_depth.store(0, Ordering::Relaxed);
            self.last_backtrace_functions.write().unwrap().clear();
        }
    }
}

// Static exception observer instance for testing
static EXCEPTION_OBSERVER: std::sync::OnceLock<TestExceptionObserver> = std::sync::OnceLock::new();

fn get_or_init_exception_observer() -> &'static TestExceptionObserver {
    EXCEPTION_OBSERVER.get_or_init(TestExceptionObserver::new)
}

/// Get the current exception count from the exception observer.
#[php_function]
pub fn exception_observer_test_get_count() -> u64 {
    get_or_init_exception_observer().get_exception_count()
}

/// Get the last exception class name.
#[php_function]
pub fn exception_observer_test_get_last_class() -> String {
    get_or_init_exception_observer().get_last_class()
}

/// Get the last exception message.
#[php_function]
pub fn exception_observer_test_get_last_message() -> String {
    get_or_init_exception_observer().get_last_message()
}

/// Get the last exception file.
#[php_function]
pub fn exception_observer_test_get_last_file() -> String {
    get_or_init_exception_observer().get_last_file()
}

/// Get the last exception line number.
#[php_function]
pub fn exception_observer_test_get_last_line() -> u64 {
    get_or_init_exception_observer().get_last_line()
}

/// Get the last exception code.
#[php_function]
pub fn exception_observer_test_get_last_code() -> i64 {
    get_or_init_exception_observer().get_last_code()
}

/// Reset the exception observer counters.
#[php_function]
pub fn exception_observer_test_reset() {
    get_or_init_exception_observer().reset();
}

/// Get the last exception backtrace depth.
#[php_function]
pub fn exception_observer_test_get_backtrace_depth() -> u64 {
    get_or_init_exception_observer().get_last_backtrace_depth()
}

/// Get the last exception backtrace function names as a comma-separated string.
#[php_function]
pub fn exception_observer_test_get_backtrace_functions() -> String {
    get_or_init_exception_observer()
        .get_last_backtrace_functions()
        .join(",")
}

/// Wrapper exception observer that delegates to the static instance.
struct TestExceptionObserverWrapper;

impl ExceptionObserver for TestExceptionObserverWrapper {
    fn on_exception(&self, exception: &ExceptionInfo) {
        get_or_init_exception_observer().on_exception(exception);
    }
}

// ============================================================================
// Module Builder
// ============================================================================

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    // Register the fcall observer factory
    let builder = builder.fcall_observer(|| TestObserverWrapper);

    // Register the error observer factory
    let builder = builder.error_observer(|| TestErrorObserverWrapper);

    // Register the exception observer factory
    let builder = builder.exception_observer(|| TestExceptionObserverWrapper);

    builder
        // Fcall observer functions
        .function(wrap_function!(observer_test_get_call_count))
        .function(wrap_function!(observer_test_get_end_count))
        .function(wrap_function!(observer_test_reset))
        // Error observer functions
        .function(wrap_function!(error_observer_test_get_error_count))
        .function(wrap_function!(error_observer_test_get_warning_count))
        .function(wrap_function!(error_observer_test_get_last_message))
        .function(wrap_function!(error_observer_test_get_last_file))
        .function(wrap_function!(error_observer_test_get_last_line))
        .function(wrap_function!(error_observer_test_reset))
        // Exception observer functions
        .function(wrap_function!(exception_observer_test_get_count))
        .function(wrap_function!(exception_observer_test_get_last_class))
        .function(wrap_function!(exception_observer_test_get_last_message))
        .function(wrap_function!(exception_observer_test_get_last_file))
        .function(wrap_function!(exception_observer_test_get_last_line))
        .function(wrap_function!(exception_observer_test_get_last_code))
        .function(wrap_function!(exception_observer_test_reset))
        .function(wrap_function!(exception_observer_test_get_backtrace_depth))
        .function(wrap_function!(
            exception_observer_test_get_backtrace_functions
        ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn observer_works() {
        assert!(crate::integration::test::run_php("observer/observer.php"));
    }

    #[test]
    fn error_observer_works() {
        assert!(crate::integration::test::run_php(
            "observer/error_observer.php"
        ));
    }

    #[test]
    fn exception_observer_works() {
        assert!(crate::integration::test::run_php(
            "observer/exception_observer.php"
        ));
    }
}
