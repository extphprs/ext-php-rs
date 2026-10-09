#![cfg_attr(windows, feature(abi_vectorcall))]
#![cfg(all(feature = "embed", feature = "observer"))]
#![allow(missing_docs)]
extern crate ext_php_rs;

use std::sync::atomic::{AtomicUsize, Ordering};

use ext_php_rs::builders::SapiBuilder;
use ext_php_rs::embed::{
    Embed, cleanup_sapi_allocations, ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup,
};
use ext_php_rs::ffi::{
    ZEND_RESULT_CODE_SUCCESS, php_module_shutdown, php_module_startup, php_request_shutdown,
    php_request_startup, sapi_shutdown, sapi_startup,
};
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::{
    ErrorInfo, ErrorObserver, ErrorType, ExceptionInfo, ExceptionObserver, ExecuteData, FcallInfo,
    FcallObserver, try_catch_first,
};

static FCALLS: AtomicUsize = AtomicUsize::new(0);
static ERRORS: AtomicUsize = AtomicUsize::new(0);
static EXCEPTIONS: AtomicUsize = AtomicUsize::new(0);

struct CallCounter;

impl FcallObserver for CallCounter {
    fn should_observe(&self, info: &FcallInfo) -> bool {
        info.function_name == Some("restart_observed")
    }

    fn begin(&self, _: &ExecuteData) {
        FCALLS.fetch_add(1, Ordering::Relaxed);
    }

    fn end(&self, _: &ExecuteData, _: Option<&Zval>) {}
}

struct ErrorCounter;

impl ErrorObserver for ErrorCounter {
    fn should_observe(&self, error_type: ErrorType) -> bool {
        error_type.contains(ErrorType::UserWarning)
    }

    fn on_error(&self, _: &ErrorInfo) {
        ERRORS.fetch_add(1, Ordering::Relaxed);
    }
}

struct ExceptionCounter;

impl ExceptionObserver for ExceptionCounter {
    fn on_exception(&self, _: &ExceptionInfo) {
        EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
    }
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .fcall_observer(|| CallCounter)
        .error_observer(|| ErrorCounter)
        .exception_observer(|| ExceptionCounter)
}

const SCRIPT: &str = "(function () { \
    function restart_observed() {} \
    restart_observed(); \
    @trigger_error('observed', E_USER_WARNING); \
    try { throw new Exception('observed'); } catch (Exception $e) {} \
})()";

fn run_module_once() -> [usize; 3] {
    let mut sapi = SapiBuilder::new("observer-restart", "Observer restart")
        .build()
        .unwrap();
    let sapi = &raw mut sapi;

    unsafe {
        ext_php_rs_sapi_startup();
        sapi_startup(sapi);
        assert_eq!(
            php_module_startup(sapi, get_module()),
            ZEND_RESULT_CODE_SUCCESS
        );
    }

    assert_eq!(unsafe { php_request_startup() }, ZEND_RESULT_CODE_SUCCESS);

    for counter in [&FCALLS, &ERRORS, &EXCEPTIONS] {
        counter.store(0, Ordering::Relaxed);
    }

    try_catch_first(|| {
        Embed::eval(SCRIPT).unwrap_or_else(|e| panic!("the script failed: {e:?}"));
    })
    .expect("PHP bailed out");

    let seen = [&FCALLS, &ERRORS, &EXCEPTIONS].map(|counter| counter.load(Ordering::Relaxed));

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
        cleanup_sapi_allocations(sapi);
    }

    seen
}

#[test]
fn observers_fire_after_every_module_restart() {
    for lifecycle in 1..=3 {
        let seen = std::thread::spawn(run_module_once)
            .join()
            .expect("the module should start, serve a request and stop");
        assert_eq!(
            seen,
            [1, 1, 1],
            "fcalls, errors, exceptions in lifecycle {lifecycle}"
        );
    }
}
