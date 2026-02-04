//! Example: Observer API for function call profiling, error tracking, and exception monitoring.
//!
//! Build: `cargo build --example observer --features observer`

#![allow(missing_docs, clippy::must_use_candidate)]
#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;
use std::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Function Call Observer
// ============================================================================

/// Simple profiler that counts user function calls.
pub struct SimpleProfiler {
    call_count: AtomicU64,
}

impl SimpleProfiler {
    fn new() -> Self {
        Self {
            call_count: AtomicU64::new(0),
        }
    }
}

impl FcallObserver for SimpleProfiler {
    fn should_observe(&self, info: &FcallInfo) -> bool {
        !info.is_internal
    }

    fn begin(&self, _execute_data: &ExecuteData) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
    }

    fn end(&self, _execute_data: &ExecuteData, _retval: Option<&Zval>) {}
}

#[php_function]
pub fn observer_get_call_count() -> u64 {
    0
}

// ============================================================================
// Error Observer
// ============================================================================

/// APM-style error tracker that counts errors by severity.
pub struct ErrorTracker {
    fatal_count: AtomicU64,
    warning_count: AtomicU64,
}

impl ErrorTracker {
    fn new() -> Self {
        Self {
            fatal_count: AtomicU64::new(0),
            warning_count: AtomicU64::new(0),
        }
    }
}

impl ErrorObserver for ErrorTracker {
    fn should_observe(&self, error_type: ErrorType) -> bool {
        (ErrorType::FATAL | ErrorType::WARNING | ErrorType::USER_WARNING).contains(error_type)
    }

    fn on_error(&self, error: &ErrorInfo) {
        if ErrorType::FATAL.contains(error.error_type) {
            self.fatal_count.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "[FATAL] {}:{} - {}",
                error.filename.unwrap_or("<unknown>"),
                error.lineno,
                error.message
            );

            if let Some(trace) = error.backtrace() {
                for frame in trace {
                    eprintln!(
                        "  at {}{}{}:{}",
                        frame.class.as_deref().unwrap_or(""),
                        if frame.class.is_some() { "::" } else { "" },
                        frame.function.as_deref().unwrap_or("<main>"),
                        frame.line
                    );
                }
            }
        } else {
            self.warning_count.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "[WARNING] {}:{} - {}",
                error.filename.unwrap_or("<unknown>"),
                error.lineno,
                error.message
            );
        }
    }
}

pub struct ExceptionTracker {
    exception_count: AtomicU64,
}

impl ExceptionTracker {
    fn new() -> Self {
        Self {
            exception_count: AtomicU64::new(0),
        }
    }
}

impl ExceptionObserver for ExceptionTracker {
    fn on_exception(&self, exception: &ExceptionInfo) {
        self.exception_count.fetch_add(1, Ordering::Relaxed);
        eprintln!(
            "[EXCEPTION] {}: {} at {}:{}",
            exception.class_name,
            exception.message.as_deref().unwrap_or("<no message>"),
            exception.file.as_deref().unwrap_or("<unknown>"),
            exception.line
        );
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .fcall_observer(SimpleProfiler::new)
        .error_observer(ErrorTracker::new)
        .exception_observer(ExceptionTracker::new)
        .function(wrap_function!(observer_get_call_count))
}
