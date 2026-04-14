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

#[php_class]
pub struct BenchProps {
    #[php(prop)]
    pub field_a: i64,
    #[php(prop)]
    pub field_b: String,
    #[php(prop)]
    pub field_c: bool,
    inner_value: i64,
}

#[php_impl]
impl BenchProps {
    pub fn __construct(a: i64, b: String) -> Self {
        Self {
            field_a: a,
            field_b: b,
            field_c: true,
            inner_value: a * 2,
        }
    }

    #[php(getter)]
    pub fn get_computed(&self) -> i64 {
        self.inner_value
    }

    #[php(setter)]
    pub fn set_computed(&mut self, val: i64) {
        self.inner_value = val;
    }
}

#[php_module]
pub fn build_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .function(wrap_function!(bench_function))
        .function(wrap_function!(bench_callback_function))
        .class::<BenchClass>()
        .class::<BenchProps>()
}
