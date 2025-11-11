//! Sapi Tests
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
use ext_php_rs::zend::try_catch_first;
use std::ffi::c_char;
use std::sync::Mutex;

#[cfg(php_zts)]
use ext_php_rs::embed::{ext_php_rs_sapi_per_thread_init, ext_php_rs_sapi_per_thread_shutdown};
#[cfg(php_zts)]
use std::sync::Arc;
#[cfg(php_zts)]
use std::thread;

static mut LAST_OUTPUT: String = String::new();

// Global mutex to ensure SAPI tests don't run concurrently. PHP does not allow
// multiple SAPIs to exist at the same time. This prevents the tests from
// overwriting each other's state.
static SAPI_TEST_MUTEX: Mutex<()> = Mutex::new(());

extern "C" fn output_tester(str: *const c_char, str_length: usize) -> usize {
    let char = unsafe { std::slice::from_raw_parts(str.cast::<u8>(), str_length) };
    let string = String::from_utf8_lossy(char);

    println!("{string}");

    unsafe {
        LAST_OUTPUT = string.to_string();
    };

    str_length
}

#[test]
fn test_sapi() {
    let _guard = SAPI_TEST_MUTEX.lock().unwrap();

    let mut builder = SapiBuilder::new("test", "Test");
    builder = builder.ub_write_function(output_tester);

    let sapi = builder.build().unwrap().into_raw();
    let module = get_module();

    unsafe {
        ext_php_rs_sapi_startup();
    }

    unsafe {
        sapi_startup(sapi);
    }

    unsafe {
        php_module_startup(sapi, module);
    }

    let result = unsafe { php_request_startup() };

    assert_eq!(result, ZEND_RESULT_CODE_SUCCESS);

    let _ = try_catch_first(|| {
        let result = Embed::eval("$foo = hello_world('foo');");

        assert!(result.is_ok());

        let zval = result.unwrap();

        assert!(zval.is_string());

        let string = zval.string().unwrap();

        assert_eq!(string.clone(), "Hello, foo!");

        let result = Embed::eval("var_dump($foo);");

        assert!(result.is_ok());
    });

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
    }

    unsafe {
        php_module_shutdown();
    }

    unsafe {
        sapi_shutdown();
    }

    unsafe {
        ext_php_rs_sapi_shutdown();
    }
}

/// Gives you a nice greeting!
///
/// @param string $name Your name.
///
/// @return string Nice greeting!
#[php_function]
pub fn hello_world(name: String) -> String {
    format!("Hello, {name}!")
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(hello_world))
}

#[test]
#[cfg(php_zts)]
fn test_sapi_multithread() {
    let _guard = SAPI_TEST_MUTEX.lock().unwrap();

    let mut builder = SapiBuilder::new("test-mt", "Test Multi-threaded");
    builder = builder.ub_write_function(output_tester);

    let sapi = builder.build().unwrap().into_raw();
    let module = get_module();

    unsafe {
        ext_php_rs_sapi_startup();
    }

    unsafe {
        sapi_startup(sapi);
    }

    unsafe {
        php_module_startup(sapi, module);
    }

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    for i in 0..4 {
        let results = Arc::clone(&results);

        let handle = thread::spawn(move || {
            unsafe {
                ext_php_rs_sapi_per_thread_init();
            }

            let result = unsafe { php_request_startup() };
            assert_eq!(result, ZEND_RESULT_CODE_SUCCESS);

            let _ = try_catch_first(|| {
                let eval_result = Embed::eval(&format!("hello_world('thread-{i}');"));

                match eval_result {
                    Ok(zval) => {
                        assert!(zval.is_string());
                        let string = zval.string().unwrap();
                        let output = string.to_string();
                        assert_eq!(output, format!("Hello, thread-{i}!"));

                        results.lock().unwrap().push((i, output));
                    }
                    Err(e) => panic!("Evaluation failed in thread {i}: {e:?}"),
                }
            });

            unsafe {
                php_request_shutdown(std::ptr::null_mut());
            }

            unsafe {
                ext_php_rs_sapi_per_thread_shutdown();
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 4);

    for i in 0..4 {
        assert!(
            results
                .iter()
                .any(|(idx, output)| { *idx == i && output == &format!("Hello, thread-{i}!") })
        );
    }

    unsafe {
        php_module_shutdown();
    }

    unsafe {
        sapi_shutdown();
    }

    unsafe {
        ext_php_rs_sapi_shutdown();
    }
}
