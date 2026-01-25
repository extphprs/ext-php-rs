//! Integration tests for the Observer API.

use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;
use std::sync::atomic::{AtomicU64, Ordering};

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

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    // Register the observer factory
    let builder = builder.fcall_observer(|| {
        // Return a wrapper that delegates to the static observer
        TestObserverWrapper
    });

    builder
        .function(wrap_function!(observer_test_get_call_count))
        .function(wrap_function!(observer_test_get_end_count))
        .function(wrap_function!(observer_test_reset))
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

#[cfg(test)]
mod tests {
    #[test]
    fn observer_works() {
        assert!(crate::integration::test::run_php("observer/observer.php"));
    }
}
