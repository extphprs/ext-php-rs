//! Example module for RustHotload::loadDir()
//!
//! This demonstrates a full Cargo project that can be loaded at runtime.

use ext_php_rs::prelude::*;

/// Greet someone with their name
#[php_function]
fn greet_user(name: String) -> String {
    format!("Hello from my_module, {}!", name)
}

/// Calculate the factorial of a number
#[php_function]
fn factorial(n: i64) -> i64 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

/// Return the current Unix timestamp in seconds
#[php_function]
fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .function(wrap_function!(greet_user))
        .function(wrap_function!(factorial))
        .function(wrap_function!(current_timestamp))
}

// Hotload plugin ABI - required for RustHotload::loadDir()
#[repr(C)]
pub struct HotloadFunctionInfo {
    pub name: *const std::ffi::c_char,
}

// SAFETY: The pointers are to static string literals which are inherently thread-safe
unsafe impl Sync for HotloadFunctionInfo {}

#[repr(C)]
pub struct HotloadInfo {
    pub name: *const std::ffi::c_char,
    pub version: *const std::ffi::c_char,
    pub num_functions: u32,
    pub functions: *const HotloadFunctionInfo,
}

// SAFETY: The pointers are to static string literals which are inherently thread-safe
unsafe impl Sync for HotloadInfo {}

static FUNCTION_INFO: [HotloadFunctionInfo; 3] = [
    HotloadFunctionInfo {
        name: b"greet_user\0".as_ptr().cast(),
    },
    HotloadFunctionInfo {
        name: b"factorial\0".as_ptr().cast(),
    },
    HotloadFunctionInfo {
        name: b"current_timestamp\0".as_ptr().cast(),
    },
];

static PLUGIN_INFO: HotloadInfo = HotloadInfo {
    name: b"my_module\0".as_ptr().cast(),
    version: b"0.1.0\0".as_ptr().cast(),
    num_functions: 3,
    functions: FUNCTION_INFO.as_ptr(),
};

#[no_mangle]
pub extern "C" fn hotload_info() -> *const HotloadInfo {
    &PLUGIN_INFO
}

#[no_mangle]
pub extern "C" fn hotload_init() {}
