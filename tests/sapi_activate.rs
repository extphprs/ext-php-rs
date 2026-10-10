#![cfg_attr(windows, feature(abi_vectorcall))]
#![cfg(feature = "embed")]
#![allow(missing_docs)]
extern crate ext_php_rs;

use std::{
    ffi::{CString, c_int},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use ext_php_rs::builders::SapiBuilder;
use ext_php_rs::embed::{
    Embed, cleanup_sapi_allocations, ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup,
};
use ext_php_rs::ffi::{
    MODULE_PERSISTENT, ZEND_RESULT_CODE_FAILURE, ZEND_RESULT_CODE_SUCCESS, php_module_shutdown,
    php_module_startup, php_request_shutdown, php_request_startup, sapi_shutdown, sapi_startup,
};
use ext_php_rs::prelude::*;
use ext_php_rs::zend::{SapiGlobals, SapiModule, SapiRequestInfo, set_header, try_catch_first};

static SAPI_ACTIVATIONS: AtomicUsize = AtomicUsize::new(0);
static SEEN_URIS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static FAIL_STARTUP: AtomicBool = AtomicBool::new(false);

extern "C" fn user_startup(_ty: i32, _module_number: i32) -> i32 {
    if FAIL_STARTUP.load(Ordering::Relaxed) {
        ZEND_RESULT_CODE_FAILURE
    } else {
        ZEND_RESULT_CODE_SUCCESS
    }
}

extern "C" fn counting_activate() -> c_int {
    SAPI_ACTIVATIONS.fetch_add(1, Ordering::Relaxed);
    ZEND_RESULT_CODE_SUCCESS
}

fn on_activate(info: &SapiRequestInfo) {
    let uri = info.request_uri().unwrap_or_default().to_owned();
    SEEN_URIS
        .lock()
        .expect("the URI list is poisoned")
        .push(uri.clone());
    assert_ne!(uri, "/panic", "the callback panics on purpose");
    set_header(&format!("X-Activate: {uri}")).expect("set_header failed in the hook");
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .startup_function(user_startup)
        .sapi_activate_function(on_activate)
}

struct Lifecycle {
    sapi_activations: usize,
    seen_uris: Vec<String>,
    headers: String,
    sapi_handler_restored: bool,
}

fn start_request(uri: &CString) {
    SapiGlobals::get_mut().request_info.request_uri = uri.as_ptr().cast_mut();
    assert_eq!(unsafe { php_request_startup() }, ZEND_RESULT_CODE_SUCCESS);
}

fn end_request() {
    unsafe { php_request_shutdown(std::ptr::null_mut()) };
    SapiGlobals::get_mut().request_info.request_uri = std::ptr::null_mut();
}

fn run_module_once() -> Lifecycle {
    let mut sapi = SapiBuilder::new("sapi-activate", "SAPI activate")
        .activate_function(counting_activate)
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

    SAPI_ACTIVATIONS.store(0, Ordering::Relaxed);
    SEEN_URIS.lock().expect("the URI list is poisoned").clear();

    let page = CString::new("/page").unwrap();
    start_request(&page);
    let headers = try_catch_first(|| {
        Embed::eval("implode('|', preg_grep('/^X-Activate/', headers_list()));")
            .ok()
            .and_then(|headers| headers.string())
            .unwrap_or_default()
    })
    .expect("PHP bailed out");
    end_request();

    let panic = CString::new("/panic").unwrap();
    start_request(&panic);
    end_request();

    let sapi_activations = SAPI_ACTIVATIONS.load(Ordering::Relaxed);
    let seen_uris = SEEN_URIS.lock().expect("the URI list is poisoned").clone();

    unsafe { php_module_shutdown() };
    let sapi_handler_restored = is_counting_activate(&SapiModule::get());

    unsafe {
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
        cleanup_sapi_allocations(sapi);
    }

    Lifecycle {
        sapi_activations,
        seen_uris,
        headers,
        sapi_handler_restored,
    }
}

fn is_counting_activate(module: &SapiModule) -> bool {
    let expected: unsafe extern "C" fn() -> c_int = counting_activate;
    module
        .activate
        .is_some_and(|current| std::ptr::fn_addr_eq(current, expected))
}

fn failed_startup_leaves_sapi_handler() -> bool {
    let mut sapi = SapiBuilder::new("sapi-activate", "SAPI activate")
        .activate_function(counting_activate)
        .build()
        .unwrap();
    let sapi = &raw mut sapi;

    unsafe {
        ext_php_rs_sapi_startup();
        sapi_startup(sapi);
        assert_eq!(
            php_module_startup(sapi, std::ptr::null_mut()),
            ZEND_RESULT_CODE_SUCCESS
        );
    }

    FAIL_STARTUP.store(true, Ordering::Relaxed);
    let startup = unsafe { (*get_module()).module_startup_func }.expect("no MINIT");
    let result = unsafe { startup(MODULE_PERSISTENT.cast_signed(), 0) };
    FAIL_STARTUP.store(false, Ordering::Relaxed);
    assert_ne!(result, ZEND_RESULT_CODE_SUCCESS);
    let untouched = is_counting_activate(&SapiModule::get());

    unsafe {
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
        cleanup_sapi_allocations(sapi);
    }

    untouched
}

#[test]
fn hook_runs_once_per_request_after_every_module_restart() {
    for lifecycle in 1..=3 {
        let seen = std::thread::spawn(run_module_once)
            .join()
            .expect("the module should start, serve two requests and stop");

        assert_eq!(
            seen.sapi_activations, 2,
            "SAPI handler in lifecycle {lifecycle}"
        );
        assert_eq!(
            seen.seen_uris,
            ["/page", "/panic"],
            "hook calls in lifecycle {lifecycle}"
        );
        assert_eq!(
            seen.headers, "X-Activate: /page",
            "headers in lifecycle {lifecycle}"
        );
        assert!(
            seen.sapi_handler_restored,
            "SAPI handler restored in lifecycle {lifecycle}"
        );
    }

    let untouched = std::thread::spawn(failed_startup_leaves_sapi_handler)
        .join()
        .expect("the engine should start and stop without the module");
    assert!(untouched, "a failed MINIT installed the hook");
}
