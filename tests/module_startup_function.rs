//! A startup function set with `ModuleBuilder::startup_function` inside
//! `#[php_module]` runs at MINIT, next to the registration the macro does.
#![cfg_attr(windows, feature(abi_vectorcall))]
#![cfg(feature = "embed")]
#![allow(missing_docs, clippy::must_use_candidate)]
extern crate ext_php_rs;

use std::sync::atomic::{AtomicU32, Ordering};

use ext_php_rs::builders::SapiBuilder;
use ext_php_rs::embed::{
    Embed, cleanup_sapi_allocations, ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup,
};
use ext_php_rs::ffi::{
    ZEND_RESULT_CODE_SUCCESS, php_module_shutdown, php_module_startup, php_request_shutdown,
    php_request_startup, sapi_shutdown, sapi_startup,
};
use ext_php_rs::prelude::*;
use ext_php_rs::zend::try_catch_first;

static BUILDER_STARTUPS: AtomicU32 = AtomicU32::new(0);

extern "C" fn builder_startup(_ty: i32, _module_number: i32) -> i32 {
    BUILDER_STARTUPS.fetch_add(1, Ordering::SeqCst);
    0
}

#[php_function]
pub fn startup_answer() -> i64 {
    42
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .startup_function(builder_startup)
        .function(wrap_function!(startup_answer))
}

#[test]
fn test_builder_startup_function_runs_at_minit() {
    let mut sapi = SapiBuilder::new("startup", "Startup").build().unwrap();
    let sapi = &raw mut sapi;

    unsafe {
        ext_php_rs_sapi_startup();
        sapi_startup(sapi);
        assert_eq!(
            php_module_startup(sapi, get_module()),
            ZEND_RESULT_CODE_SUCCESS
        );
    }

    assert_eq!(BUILDER_STARTUPS.load(Ordering::SeqCst), 1);

    assert_eq!(unsafe { php_request_startup() }, ZEND_RESULT_CODE_SUCCESS);

    try_catch_first(|| {
        let answer = Embed::eval("startup_answer();").expect("startup_answer() failed");
        assert_eq!(answer.long(), Some(42));
    })
    .expect("PHP bailed out");

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
        cleanup_sapi_allocations(sapi);
    }
}
