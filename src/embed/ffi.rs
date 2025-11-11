//! Raw FFI bindings to the Zend API.

#![allow(clippy::all)]
#![allow(warnings)]

#[cfg(php82)]
use crate::ffi::php_ini_builder;

use std::ffi::{c_char, c_int, c_void};

#[link(name = "wrapper")]
unsafe extern "C" {
    pub fn ext_php_rs_embed_callback(
        argc: c_int,
        argv: *mut *mut c_char,
        func: unsafe extern "C" fn(*const c_void) -> *const c_void,
        ctx: *const c_void,
    ) -> *mut c_void;

    pub fn ext_php_rs_sapi_startup();
    pub fn ext_php_rs_sapi_shutdown();
    pub fn ext_php_rs_sapi_per_thread_init();
    pub fn ext_php_rs_sapi_per_thread_shutdown();

    pub fn ext_php_rs_php_error(
        type_: ::std::os::raw::c_int,
        error_msg: *const ::std::os::raw::c_char,
        ...
    );

    #[cfg(php82)]
    pub fn ext_php_rs_php_ini_builder_deinit(builder: *mut php_ini_builder);
}
