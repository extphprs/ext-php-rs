//! Example: Observer API for function call profiling.
//!
//! Build: `cargo build --example observer --features observer`

#![allow(missing_docs, clippy::must_use_candidate)]
#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;
use std::sync::atomic::{AtomicU64, Ordering};

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

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .fcall_observer(SimpleProfiler::new)
        .function(wrap_function!(observer_get_call_count))
}
