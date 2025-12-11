#![cfg_attr(windows, feature(abi_vectorcall))]
#![allow(
    clippy::must_use_candidate,
    clippy::missing_panics_doc,
    clippy::needless_pass_by_value,
    clippy::implicit_hasher
)]

use ext_php_rs::{prelude::*, types::Zval};

#[php_function]
pub fn bench_function(n: u64) -> u64 {
    // A simple function that does not do much work
    n
}

#[php_function]
pub fn bench_callback_function(callback: ZendCallable) -> Zval {
    // Call the provided PHP callable with a fixed argument
    callback
        .try_call(vec![&42])
        .expect("Failed to call function")
}

#[php_function]
pub fn start_instrumentation() {
    gungraun::client_requests::callgrind::start_instrumentation();
    // gungraun::client_requests::callgrind::toggle_collect();
}

#[php_function]
pub fn stop_instrumentation() {
    gungraun::client_requests::callgrind::stop_instrumentation();
}

#[php_module]
pub fn build_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .function(wrap_function!(bench_function))
        .function(wrap_function!(bench_callback_function))
        .function(wrap_function!(start_instrumentation))
        .function(wrap_function!(stop_instrumentation))
}
