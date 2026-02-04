//! Integration tests for raw function attribute (#[php(raw)])
//! and `ExecuteData::num_args()` / `get_arg()` methods.

#![cfg_attr(windows, feature(abi_vectorcall))]
#![cfg(feature = "embed")]
#![allow(
    missing_docs,
    clippy::needless_pass_by_value,
    clippy::must_use_candidate
)]

extern crate ext_php_rs;

use ext_php_rs::builders::SapiBuilder;
use ext_php_rs::embed::{Embed, ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup};
use ext_php_rs::ffi::{
    ZEND_RESULT_CODE_SUCCESS, php_module_shutdown, php_module_startup, php_request_shutdown,
    php_request_startup, sapi_shutdown, sapi_startup,
};
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::{ExecuteData, try_catch_first};
use std::ffi::c_char;
use std::sync::Mutex;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

extern "C" fn output_handler(str: *const c_char, str_length: usize) -> usize {
    let _ = unsafe { std::slice::from_raw_parts(str.cast::<u8>(), str_length) };
    str_length
}

// ============================================================================
// Standard #[php_function] for comparison
// ============================================================================

#[php_function]
pub fn raw_test_standard_add(a: i64, b: i64) -> i64 {
    a + b
}

// ============================================================================
// Raw function implementations using #[php(raw)]
// ============================================================================

/// Raw function - zero overhead, direct access
#[php_function]
#[php(raw)]
pub fn raw_test_noop(_ex: &mut ExecuteData, retval: &mut Zval) {
    retval.set_long(42);
}

/// Raw function with argument access
#[php_function]
#[php(raw)]
pub fn raw_test_add(ex: &mut ExecuteData, retval: &mut Zval) {
    let a = unsafe { ex.get_arg(0) }
        .and_then(|zv| zv.long())
        .unwrap_or(0);
    let b = unsafe { ex.get_arg(1) }
        .and_then(|zv| zv.long())
        .unwrap_or(0);
    retval.set_long(a + b);
}

/// Raw function that uses `num_args()`
#[php_function]
#[php(raw)]
pub fn raw_test_count_args(ex: &mut ExecuteData, retval: &mut Zval) {
    retval.set_long(i64::from(ex.num_args()));
}

/// Raw function that sums all arguments
#[php_function]
#[php(raw)]
pub fn raw_test_sum_all(ex: &mut ExecuteData, retval: &mut Zval) {
    let n = ex.num_args();
    let mut sum: i64 = 0;
    for i in 0..n {
        if let Some(zv) = unsafe { ex.get_arg(i as usize) } {
            sum += zv.long().unwrap_or(0);
        }
    }
    retval.set_long(sum);
}

// ============================================================================
// Module registration
// ============================================================================

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .function(wrap_function!(raw_test_standard_add))
        .function(wrap_function!(raw_test_noop))
        .function(wrap_function!(raw_test_add))
        .function(wrap_function!(raw_test_count_args))
        .function(wrap_function!(raw_test_sum_all))
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_raw_noop() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let builder =
        SapiBuilder::new("raw-test1", "Raw Function Test 1").ub_write_function(output_handler);
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
        let result = Embed::eval("raw_test_noop();");
        assert!(result.is_ok());
        let zval = result.unwrap();
        assert_eq!(zval.long(), Some(42));
    });

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
    }
}

#[test]
fn test_raw_add() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let builder =
        SapiBuilder::new("raw-test2", "Raw Function Test 2").ub_write_function(output_handler);
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
        let result = Embed::eval("raw_test_add(10, 32);");
        assert!(result.is_ok());
        let zval = result.unwrap();
        assert_eq!(zval.long(), Some(42));
    });

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
    }
}

#[test]
fn test_raw_count_args() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let builder =
        SapiBuilder::new("raw-test3", "Raw Function Test 3").ub_write_function(output_handler);
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
        // Test with 0 args
        let result = Embed::eval("raw_test_count_args();");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().long(), Some(0));

        // Test with 3 args
        let result = Embed::eval("raw_test_count_args(1, 2, 3);");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().long(), Some(3));

        // Test with 5 args
        let result = Embed::eval("raw_test_count_args(1, 2, 3, 4, 5);");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().long(), Some(5));
    });

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
    }
}

#[test]
fn test_raw_sum_all() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let builder =
        SapiBuilder::new("raw-test4", "Raw Function Test 4").ub_write_function(output_handler);
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
        // Test sum of multiple args
        let result = Embed::eval("raw_test_sum_all(1, 2, 3, 4, 5);");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().long(), Some(15));

        // Test with no args
        let result = Embed::eval("raw_test_sum_all();");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().long(), Some(0));

        // Test with single arg
        let result = Embed::eval("raw_test_sum_all(42);");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().long(), Some(42));
    });

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
    }
}

#[test]
fn test_standard_vs_raw_equivalence() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let builder =
        SapiBuilder::new("raw-test5", "Raw Function Test 5").ub_write_function(output_handler);
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
        // Both should return the same result
        let standard = Embed::eval("raw_test_standard_add(10, 32);").unwrap();
        let raw = Embed::eval("raw_test_add(10, 32);").unwrap();

        assert_eq!(standard.long(), raw.long());
        assert_eq!(standard.long(), Some(42));
    });

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
    }
}
