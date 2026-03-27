#![cfg_attr(windows, feature(abi_vectorcall))]
#![allow(
    clippy::must_use_candidate,
    clippy::missing_panics_doc,
    clippy::needless_pass_by_value,
    clippy::implicit_hasher
)]

use ext_php_rs::prelude::*;

#[php_function]
pub fn bench_function(n: u64) -> u64 {
    n
}

#[php_function]
pub fn bench_callback_function(callback: ZendCallable, n: usize) {
    for i in 0..n {
        callback
            .try_call(vec![&i])
            .expect("Failed to call function");
    }
}

#[php_function]
pub fn bench_cached_callback_function(callback: ZendCallable, n: usize) {
    let cached = callback.cache().expect("Failed to cache callback");
    for i in 0..n {
        cached
            .try_call(vec![&i])
            .expect("Failed to call cached callback");
    }
}

#[php_class]
pub struct BenchClass;

#[php_impl]
impl BenchClass {
    pub fn __construct() -> Self {
        Self
    }

    pub fn method(&self, n: u64) -> u64 {
        n
    }

    pub fn static_method(n: u64) -> u64 {
        n
    }
}

#[php_module]
pub fn build_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .function(wrap_function!(bench_function))
        .function(wrap_function!(bench_callback_function))
        .function(wrap_function!(bench_cached_callback_function))
        .class::<BenchClass>()
}
