//! Integration tests for `CachedCallable` functionality.

#![cfg_attr(windows, feature(abi_vectorcall))]
#![cfg(feature = "embed")]
#![allow(
    missing_docs,
    clippy::needless_pass_by_value,
    clippy::must_use_candidate
)]

extern crate ext_php_rs;

use ext_php_rs::builders::SapiBuilder;
use ext_php_rs::embed::{ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup};
use ext_php_rs::ffi::{
    ZEND_RESULT_CODE_SUCCESS, php_module_shutdown, php_module_startup, php_request_shutdown,
    php_request_startup, sapi_shutdown, sapi_startup,
};
use ext_php_rs::prelude::*;
use ext_php_rs::types::CachedCallable;
use ext_php_rs::zend::try_catch_first;
use std::ffi::c_char;
use std::sync::Mutex;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

extern "C" fn output_handler(str: *const c_char, str_length: usize) -> usize {
    let _ = unsafe { std::slice::from_raw_parts(str.cast::<u8>(), str_length) };
    str_length
}

// ============================================================================
// Test functions for CachedCallable
// ============================================================================

#[php_function]
pub fn cached_test_add(a: i64, b: i64) -> i64 {
    a + b
}

#[php_function]
pub fn cached_test_concat(a: String, b: String) -> String {
    format!("{a}{b}")
}

// ============================================================================
// Module registration
// ============================================================================

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .function(wrap_function!(cached_test_add))
        .function(wrap_function!(cached_test_concat))
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_cached_callable_builtin() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let builder =
        SapiBuilder::new("cached-test1", "CachedCallable Test 1").ub_write_function(output_handler);
    let sapi = builder.build().unwrap().into_raw();
    let module = get_module();

    unsafe {
        ext_php_rs_sapi_startup();
        sapi_startup(sapi);
        php_module_startup(sapi, module);
    }

    let result = unsafe { php_request_startup() };
    assert_eq!(result, ZEND_RESULT_CODE_SUCCESS);

    let _ = try_catch_first(|| {
        // Test CachedCallable with a standard PHP function
        let mut callable = CachedCallable::try_from_name("strtoupper").unwrap();

        let result = callable.try_call(vec![&"hello"]).unwrap();
        assert_eq!(result.string().unwrap().clone(), "HELLO");

        // Call again to test caching
        let result = callable.try_call(vec![&"world"]).unwrap();
        assert_eq!(result.string().unwrap().clone(), "WORLD");
    });

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
    }
}

#[test]
fn test_cached_callable_custom_function() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let builder =
        SapiBuilder::new("cached-test2", "CachedCallable Test 2").ub_write_function(output_handler);
    let sapi = builder.build().unwrap().into_raw();
    let module = get_module();

    unsafe {
        ext_php_rs_sapi_startup();
        sapi_startup(sapi);
        php_module_startup(sapi, module);
    }

    let result = unsafe { php_request_startup() };
    assert_eq!(result, ZEND_RESULT_CODE_SUCCESS);

    let _ = try_catch_first(|| {
        // Test CachedCallable with our custom function
        let mut callable = CachedCallable::try_from_name("cached_test_add").unwrap();

        let result = callable.try_call(vec![&10i64, &32i64]).unwrap();
        assert_eq!(result.long(), Some(42));

        // Call multiple times to verify caching works
        for i in 0..5 {
            let result = callable
                .try_call(vec![&i64::from(i), &i64::from(i)])
                .unwrap();
            assert_eq!(result.long(), Some(i64::from(i * 2)));
        }
    });

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
    }
}

#[test]
fn test_cached_callable_string_function() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let builder =
        SapiBuilder::new("cached-test3", "CachedCallable Test 3").ub_write_function(output_handler);
    let sapi = builder.build().unwrap().into_raw();
    let module = get_module();

    unsafe {
        ext_php_rs_sapi_startup();
        sapi_startup(sapi);
        php_module_startup(sapi, module);
    }

    let result = unsafe { php_request_startup() };
    assert_eq!(result, ZEND_RESULT_CODE_SUCCESS);

    let _ = try_catch_first(|| {
        let mut callable = CachedCallable::try_from_name("cached_test_concat").unwrap();

        let result = callable.try_call(vec![&"Hello, ", &"World!"]).unwrap();
        assert_eq!(result.string().unwrap().clone(), "Hello, World!");
    });

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
    }
}

#[test]
fn test_cached_callable_invalid_name() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let builder =
        SapiBuilder::new("cached-test4", "CachedCallable Test 4").ub_write_function(output_handler);
    let sapi = builder.build().unwrap().into_raw();
    let module = get_module();

    unsafe {
        ext_php_rs_sapi_startup();
        sapi_startup(sapi);
        php_module_startup(sapi, module);
    }

    let result = unsafe { php_request_startup() };
    assert_eq!(result, ZEND_RESULT_CODE_SUCCESS);

    let _ = try_catch_first(|| {
        // Test with non-existent function
        let result = CachedCallable::try_from_name("nonexistent_function_xyz");
        assert!(result.is_err());
    });

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
    }
}
