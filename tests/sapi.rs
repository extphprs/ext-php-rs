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
use ext_php_rs::embed::{
    Embed, RequestInfo, Sapi, SendHeadersResult, ServerContext, ServerVarRegistrar,
    ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup, worker_request_shutdown,
    worker_request_startup, worker_reset_superglobals,
};
use ext_php_rs::ffi::{
    ZEND_RESULT_CODE_SUCCESS, php_module_shutdown, php_module_startup, php_request_shutdown,
    php_request_startup, sapi_header_struct, sapi_headers_struct, sapi_shutdown, sapi_startup,
};
use ext_php_rs::prelude::*;
use ext_php_rs::zend::try_catch_first;
use std::ffi::{c_char, c_void};
use std::sync::Mutex;

#[cfg(php_zts)]
use ext_php_rs::embed::{
    PhpThreadGuard, ext_php_rs_sapi_per_thread_init, ext_php_rs_sapi_per_thread_shutdown,
};
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
                        let output = string.clone();
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

extern "C" fn register_vars(vars: *mut ext_php_rs::types::Zval) {
    let mut registrar = unsafe { ServerVarRegistrar::from_raw(vars) };
    registrar.register("SERVER_SOFTWARE", "registrar-test/1.0");
    registrar.register("REQUEST_METHOD", "POST");
}

// --- Test context and SAPI trait implementations ---

struct TestContext {
    output: Vec<u8>,
    finished: bool,
}

impl TestContext {
    fn new() -> Self {
        Self {
            output: Vec::new(),
            finished: false,
        }
    }
}

impl ServerContext for TestContext {
    fn init_request_info(&self, info: &mut RequestInfo) {
        info.request_method = Some("GET".to_string());
        info.request_uri = Some("/test".to_string());
    }

    fn read_post(&mut self, _buf: &mut [u8]) -> usize {
        0
    }

    fn read_cookies(&self) -> Option<&str> {
        None
    }

    fn finish_request(&mut self) -> bool {
        if self.finished {
            return false;
        }
        self.finished = true;
        true
    }

    fn is_request_finished(&self) -> bool {
        self.finished
    }
}

struct TestSapi;

impl Sapi for TestSapi {
    type Context = TestContext;

    fn name() -> &'static str {
        "test-sapi-trait"
    }

    fn pretty_name() -> &'static str {
        "Test SAPI Trait"
    }

    fn ub_write(ctx: &mut TestContext, buf: &[u8]) -> usize {
        ctx.output.extend_from_slice(buf);
        buf.len()
    }

    fn log_message(msg: &str, _syslog_type: i32) {
        eprintln!("[test-sapi] {msg}");
    }

    fn send_headers(
        _ctx: &mut TestContext,
        _headers: *mut sapi_headers_struct,
    ) -> SendHeadersResult {
        SendHeadersResult::SentSuccessfully
    }

    fn send_header(_ctx: &mut TestContext, _header: *mut sapi_header_struct) {}

    fn register_server_variables(_ctx: &mut TestContext, registrar: &mut ServerVarRegistrar) {
        registrar.register("SERVER_SOFTWARE", "test-sapi/1.0");
    }
}

#[test]
#[cfg(php_zts)]
fn test_php_thread_guard_drop() {
    let _guard = SAPI_TEST_MUTEX.lock().unwrap();

    let mut builder = SapiBuilder::new("test-guard", "Test Guard");
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

    let handle = thread::spawn(|| {
        let _thread_guard = PhpThreadGuard::new();

        let result = unsafe { php_request_startup() };
        assert_eq!(result, ZEND_RESULT_CODE_SUCCESS);

        let _ = try_catch_first(|| {
            let eval_result = Embed::eval("'guard_ok';");
            assert!(eval_result.is_ok());
            let zval = eval_result.unwrap();
            assert!(zval.is_string());
            assert_eq!(zval.string().unwrap(), "guard_ok");
        });

        unsafe {
            php_request_shutdown(std::ptr::null_mut());
        }
    });

    handle.join().expect("Thread panicked");

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

#[test]
fn test_server_var_registrar() {
    let _guard = SAPI_TEST_MUTEX.lock().unwrap();

    let builder = SapiBuilder::new("test-registrar", "Test Registrar")
        .ub_write_function(output_tester)
        .register_server_variables_function(register_vars);

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
        let result = Embed::eval("$_SERVER['SERVER_SOFTWARE'];");
        assert!(result.is_ok());
        let zval = result.unwrap();
        assert!(zval.is_string());
        assert_eq!(zval.string().unwrap(), "registrar-test/1.0");

        let result = Embed::eval("$_SERVER['REQUEST_METHOD'];");
        assert!(result.is_ok());
        let zval = result.unwrap();
        assert!(zval.is_string());
        assert_eq!(zval.string().unwrap(), "POST");
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

#[test]
fn test_sapi_trait_lifecycle() {
    let _guard = SAPI_TEST_MUTEX.lock().unwrap();

    let sapi = TestSapi::build_module().unwrap().into_raw();
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
        let result = Embed::eval("hello_world('trait');");
        assert!(result.is_ok());
        let zval = result.unwrap();
        assert!(zval.is_string());
        assert_eq!(zval.string().unwrap(), "Hello, trait!");
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

#[test]
fn test_worker_request_cycle() {
    let _guard = SAPI_TEST_MUTEX.lock().unwrap();

    let mut builder = SapiBuilder::new("test-worker", "Test Worker");
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
        let result = Embed::eval("$x = 42;");
        assert!(result.is_ok());
    });

    worker_request_shutdown();
    worker_request_startup().expect("worker startup failed");
    worker_reset_superglobals();

    let _ = try_catch_first(|| {
        let result = Embed::eval("isset($x) ? 'exists' : 'gone';");
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

#[test]
#[cfg(php_zts)]
fn test_full_sapi_worker_flow() {
    let _guard = SAPI_TEST_MUTEX.lock().unwrap();

    let sapi = TestSapi::build_module().unwrap().into_raw();
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

    let handle = thread::spawn(move || {
        let _thread_guard = PhpThreadGuard::new();

        let mut ctx = TestContext::new();
        let ctx_ptr: *mut c_void = (&raw mut ctx).cast::<c_void>();

        unsafe {
            let globals = &mut *ext_php_rs::ffi::ext_php_rs_sapi_globals();
            globals.server_context = ctx_ptr;
        }

        let result = unsafe { php_request_startup() };
        assert_eq!(result, ZEND_RESULT_CODE_SUCCESS);

        let _ = try_catch_first(|| {
            let result = Embed::eval("hello_world('req1');");
            assert!(result.is_ok());
            assert_eq!(result.unwrap().string().unwrap(), "Hello, req1!");
        });

        worker_request_shutdown();
        worker_request_startup().expect("worker startup failed");
        worker_reset_superglobals();

        let _ = try_catch_first(|| {
            let result = Embed::eval("hello_world('req2');");
            assert!(result.is_ok());
            assert_eq!(result.unwrap().string().unwrap(), "Hello, req2!");
        });

        unsafe {
            let globals = &mut *ext_php_rs::ffi::ext_php_rs_sapi_globals();
            globals.server_context = std::ptr::null_mut();
        }

        unsafe {
            php_request_shutdown(std::ptr::null_mut());
        }
    });

    handle.join().expect("Thread panicked");

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
