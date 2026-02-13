//! Zend Extension API bindings for low-level engine hooks.
//!
//! Enables building profilers, APMs, and code coverage tools by registering
//! as a `zend_extension` alongside the regular PHP extension.
//!
//! # Example
//!
//! ```ignore
//! use ext_php_rs::prelude::*;
//! use ext_php_rs::ffi::zend_op_array;
//!
//! struct MyProfiler;
//!
//! impl ZendExtensionHandler for MyProfiler {
//!     fn op_array_handler(&self, _op_array: &mut zend_op_array) {}
//!     fn statement_handler(&self, _execute_data: &ExecuteData) {}
//! }
//! ```

use std::ffi::{CString, c_void};
use std::ptr;
use std::sync::OnceLock;

use crate::ffi;
use crate::zend::ExecuteData;

/// Trait for handling low-level Zend Engine hooks via `zend_extension`.
///
/// All methods have default no-op implementations. Override only what you need.
///
/// # Thread Safety
///
/// Handler must be `Send + Sync`. Use thread-safe primitives for mutable state.
pub trait ZendExtensionHandler: Send + Sync + 'static {
    /// Called after compilation of each function/method `op_array`.
    /// Use to instrument compiled bytecode.
    fn op_array_handler(&self, _op_array: &mut ffi::zend_op_array) {}

    /// Called for each executed statement.
    /// Use for line-level profiling or code coverage.
    fn statement_handler(&self, _execute_data: &ExecuteData) {}

    /// Called at the beginning of each function call (legacy hook).
    fn fcall_begin_handler(&self, _execute_data: &ExecuteData) {}

    /// Called at the end of each function call (legacy hook).
    fn fcall_end_handler(&self, _execute_data: &ExecuteData) {}

    /// Called when a new `op_array` is constructed.
    /// Use to attach per-function profiling data.
    fn op_array_ctor(&self, _op_array: &mut ffi::zend_op_array) {}

    /// Called when an `op_array` is destroyed.
    /// Use to clean up per-function data.
    fn op_array_dtor(&self, _op_array: &mut ffi::zend_op_array) {}

    /// Called when another `zend_extension` is loaded or sends a message.
    fn message_handler(&self, _message: i32, _arg: *mut c_void) {}

    /// Per-request activation (distinct from RINIT).
    fn activate(&self) {}

    /// Per-request deactivation (distinct from RSHUTDOWN).
    fn deactivate(&self) {}
}

type ZendExtHandlerFactory = Box<dyn Fn() -> Box<dyn ZendExtensionHandler> + Send + Sync>;

static ZEND_EXT_FACTORY: OnceLock<ZendExtHandlerFactory> = OnceLock::new();
static ZEND_EXT_INSTANCE: OnceLock<Box<dyn ZendExtensionHandler>> = OnceLock::new();

fn get_handler() -> Option<&'static dyn ZendExtensionHandler> {
    ZEND_EXT_INSTANCE.get().map(std::convert::AsRef::as_ref)
}

// ============================================================================
// extern "C" dispatchers
// ============================================================================

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_op_array_handler(op_array: *mut ffi::zend_op_array) {
    if let Some(handler) = get_handler()
        && let Some(op) = unsafe { op_array.as_mut() }
    {
        handler.op_array_handler(op);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_statement_handler(execute_data: *mut ffi::zend_execute_data) {
    if let Some(handler) = get_handler()
        && let Some(ex) = unsafe { execute_data.as_ref() }
    {
        handler.statement_handler(ex);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_fcall_begin_handler(execute_data: *mut ffi::zend_execute_data) {
    if let Some(handler) = get_handler()
        && let Some(ex) = unsafe { execute_data.as_ref() }
    {
        handler.fcall_begin_handler(ex);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_fcall_end_handler(execute_data: *mut ffi::zend_execute_data) {
    if let Some(handler) = get_handler()
        && let Some(ex) = unsafe { execute_data.as_ref() }
    {
        handler.fcall_end_handler(ex);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_op_array_ctor(op_array: *mut ffi::zend_op_array) {
    if let Some(handler) = get_handler()
        && let Some(op) = unsafe { op_array.as_mut() }
    {
        handler.op_array_ctor(op);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_op_array_dtor(op_array: *mut ffi::zend_op_array) {
    if let Some(handler) = get_handler()
        && let Some(op) = unsafe { op_array.as_mut() }
    {
        handler.op_array_dtor(op);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_message_handler(message: i32, arg: *mut c_void) {
    if let Some(handler) = get_handler() {
        handler.message_handler(message, arg);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_activate() {
    if let Some(handler) = get_handler() {
        handler.activate();
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_deactivate() {
    if let Some(handler) = get_handler() {
        handler.deactivate();
    }
}

// ============================================================================
// Registration
// ============================================================================

// PHP compiler flags (from zend_compile.h) that cause the compiler to emit
// the special opcodes consumed by zend_extension hooks.
const ZEND_COMPILE_EXTENDED_STMT: u32 = 1 << 0;
const ZEND_COMPILE_EXTENDED_FCALL: u32 = 1 << 1;
const ZEND_COMPILE_HANDLE_OP_ARRAY: u32 = 1 << 2;

/// # Panics
///
/// Panics if called more than once.
pub(crate) fn register_zend_extension_factory(factory: ZendExtHandlerFactory) {
    assert!(
        ZEND_EXT_FACTORY.set(factory).is_ok(),
        "zend_extension_handler can only be registered once per extension"
    );
}

/// # Safety
///
/// Must be called during MINIT phase only.
pub(crate) unsafe fn zend_extension_startup(name: &str, version: &str) {
    let Some(factory) = ZEND_EXT_FACTORY.get() else {
        return;
    };

    if ZEND_EXT_INSTANCE.set(factory()).is_err() {
        return;
    }

    let c_name = CString::new(name).unwrap_or_default();
    let c_version = CString::new(version).unwrap_or_default();

    let mut ext: ffi::zend_extension = unsafe { std::mem::zeroed() };
    ext.name = c_name.into_raw();
    ext.version = c_version.into_raw();
    ext.author = ptr::null();
    ext.URL = ptr::null();
    ext.copyright = ptr::null();
    ext.startup = None;
    ext.shutdown = None;
    ext.activate = Some(ext_activate);
    ext.deactivate = Some(ext_deactivate);
    ext.message_handler = Some(ext_message_handler);
    ext.op_array_handler = Some(ext_op_array_handler);
    ext.statement_handler = Some(ext_statement_handler);
    ext.fcall_begin_handler = Some(ext_fcall_begin_handler);
    ext.fcall_end_handler = Some(ext_fcall_end_handler);
    ext.op_array_ctor = Some(ext_op_array_ctor);
    ext.op_array_dtor = Some(ext_op_array_dtor);

    unsafe {
        ffi::zend_register_extension(&raw mut ext, ptr::null_mut());

        // zend_startup_extensions_mechanism() runs before MINIT, so it won't
        // have seen our extension. Manually set the compiler flags so PHP
        // emits the opcodes that drive statement/fcall hooks.
        let cg = ffi::ext_php_rs_compiler_globals();
        (*cg).compiler_options |=
            ZEND_COMPILE_EXTENDED_STMT | ZEND_COMPILE_EXTENDED_FCALL | ZEND_COMPILE_HANDLE_OP_ARRAY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        handle_ops: bool,
    }

    unsafe impl Send for TestHandler {}
    unsafe impl Sync for TestHandler {}

    impl ZendExtensionHandler for TestHandler {
        fn op_array_handler(&self, _op_array: &mut ffi::zend_op_array) {}

        fn statement_handler(&self, _execute_data: &ExecuteData) {}
    }

    #[test]
    fn test_trait_default_methods_compile() {
        let handler = TestHandler { handle_ops: true };
        assert!(handler.handle_ops);
    }

    #[test]
    fn test_trait_impl() {
        let handler = TestHandler { handle_ops: true };
        assert!(handler.handle_ops);

        let handler = TestHandler { handle_ops: false };
        assert!(!handler.handle_ops);
    }
}
