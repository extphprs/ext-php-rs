//! Example: Zend Extension hooks for low-level profiling.
//!
//! Build: `cargo build --example zend_extension --features observer`

#![allow(missing_docs, clippy::must_use_candidate)]
#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::ffi::zend_op_array;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::ExecuteData;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct StatementProfiler {
    compiled_functions: AtomicU64,
    executed_statements: AtomicU64,
}

impl StatementProfiler {
    fn new() -> Self {
        Self {
            compiled_functions: AtomicU64::new(0),
            executed_statements: AtomicU64::new(0),
        }
    }
}

impl ZendExtensionHandler for StatementProfiler {
    fn op_array_handler(&self, _op_array: &mut zend_op_array) {
        self.compiled_functions.fetch_add(1, Ordering::Relaxed);
    }

    fn statement_handler(&self, _execute_data: &ExecuteData) {
        self.executed_statements.fetch_add(1, Ordering::Relaxed);
    }

    fn activate(&self) {
        self.executed_statements.store(0, Ordering::Relaxed);
    }
}

#[php_function]
pub fn zend_ext_compiled_count() -> u64 {
    0
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .zend_extension_handler(StatementProfiler::new)
        .function(wrap_function!(zend_ext_compiled_count))
}
