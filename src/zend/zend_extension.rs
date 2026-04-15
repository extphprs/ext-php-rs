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
//!     fn on_op_array_compiled(&self, _op_array: &mut zend_op_array) {}
//!     fn on_statement(&self, _execute_data: &ExecuteData) {}
//! }
//! ```

use std::ffi::{CString, c_void};
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
    /// Enabled by [`ZendExtensionBuilder::hook_op_array_compile`].
    fn on_op_array_compiled(&self, _op_array: &mut ffi::zend_op_array) {}

    /// Called for each executed statement.
    /// Enabled by [`ZendExtensionBuilder::hook_statements`].
    fn on_statement(&self, _execute_data: &ExecuteData) {}

    /// Called at the beginning of each function call (legacy hook).
    /// Enabled by [`ZendExtensionBuilder::hook_fcalls`].
    fn on_fcall_begin(&self, _execute_data: &ExecuteData) {}

    /// Called at the end of each function call (legacy hook).
    /// Enabled by [`ZendExtensionBuilder::hook_fcalls`].
    fn on_fcall_end(&self, _execute_data: &ExecuteData) {}

    /// Called when a new `op_array` is constructed.
    fn on_op_array_ctor(&self, _op_array: &mut ffi::zend_op_array) {}

    /// Called when an `op_array` is destroyed.
    fn on_op_array_dtor(&self, _op_array: &mut ffi::zend_op_array) {}

    /// Called when another `zend_extension` sends a message.
    fn on_message(&self, _message: i32, _arg: *mut c_void) {}

    /// Per-request activation (distinct from RINIT).
    fn on_activate(&self) {}

    /// Per-request deactivation (distinct from RSHUTDOWN).
    fn on_deactivate(&self) {}
}

// ============================================================================
// Config + statics
// ============================================================================

pub(crate) struct ZendExtensionConfig {
    pub(crate) factory: Box<dyn Fn() -> Box<dyn ZendExtensionHandler> + Send + Sync>,
    pub(crate) hook_op_array: bool,
    pub(crate) hook_statements: bool,
    pub(crate) hook_fcalls: bool,
}

static ZEND_EXT_CONFIG: OnceLock<ZendExtensionConfig> = OnceLock::new();
static ZEND_EXT_INSTANCE: OnceLock<Box<dyn ZendExtensionHandler>> = OnceLock::new();
static EXT_NAME: OnceLock<CString> = OnceLock::new();
static EXT_VERSION: OnceLock<CString> = OnceLock::new();

fn get_handler() -> Option<&'static dyn ZendExtensionHandler> {
    ZEND_EXT_INSTANCE.get().map(std::convert::AsRef::as_ref)
}

// PHP compiler flags (from zend_compile.h) that cause the compiler to emit
// the special opcodes consumed by zend_extension hooks.
const ZEND_COMPILE_EXTENDED_STMT: u32 = 1 << 0;
const ZEND_COMPILE_EXTENDED_FCALL: u32 = 1 << 1;
const ZEND_COMPILE_HANDLE_OP_ARRAY: u32 = 1 << 2;

fn compile_flags_for(cfg: &ZendExtensionConfig) -> u32 {
    let mut flags = 0u32;
    if cfg.hook_op_array {
        flags |= ZEND_COMPILE_HANDLE_OP_ARRAY;
    }
    if cfg.hook_statements {
        flags |= ZEND_COMPILE_EXTENDED_STMT;
    }
    if cfg.hook_fcalls {
        flags |= ZEND_COMPILE_EXTENDED_FCALL;
    }
    flags
}

// ============================================================================
// Builder
// ============================================================================

/// Builder for a `zend_extension` registration.
///
/// Returned by [`crate::builders::ModuleBuilder::zend_extension`]. Call
/// [`Self::finish`] to return to the outer
/// [`ModuleBuilder`](crate::builders::ModuleBuilder) after selecting opt-in
/// hooks.
///
/// Each opt-in method enables one family of [`ZendExtensionHandler`] hooks.
/// [`Self::hook_statements`] and [`Self::hook_fcalls`] also flip the matching
/// `ZEND_COMPILE_*` flag so PHP emits the extra opcodes the hook depends on,
/// which costs every compiled script. [`Self::hook_op_array_compile`] has no
/// compile-time cost because its flag is already set by PHP's defaults; it
/// only controls whether the dispatcher callback runs.
#[must_use = "call .finish() to return to ModuleBuilder"]
pub struct ZendExtensionBuilder<'a> {
    module: Option<crate::builders::ModuleBuilder<'a>>,
    factory: Box<dyn Fn() -> Box<dyn ZendExtensionHandler> + Send + Sync>,
    hook_op_array: bool,
    hook_statements: bool,
    hook_fcalls: bool,
}

impl<'a> ZendExtensionBuilder<'a> {
    pub(crate) fn new<F, H>(module: crate::builders::ModuleBuilder<'a>, factory: F) -> Self
    where
        F: Fn() -> H + Send + Sync + 'static,
        H: ZendExtensionHandler,
    {
        Self {
            module: Some(module),
            factory: Box::new(move || Box::new(factory())),
            hook_op_array: false,
            hook_statements: false,
            hook_fcalls: false,
        }
    }

    /// Enable `on_op_array_compiled`, called after PHP finishes compiling
    /// each function or method.
    ///
    /// Unlike [`Self::hook_statements`] and [`Self::hook_fcalls`], this
    /// opt-in does not change the bytecode PHP emits: the matching flag
    /// (`ZEND_COMPILE_HANDLE_OP_ARRAY`) is already part of PHP's default
    /// `CG(compiler_options)`. The opt-in only controls whether the
    /// dispatcher is registered, so the cost scales with the number of
    /// compiled functions, not with every opcode.
    pub fn hook_op_array_compile(mut self) -> Self {
        self.hook_op_array = true;
        self
    }

    /// Enable `on_statement` -- called for every executed statement.
    ///
    /// Flips `ZEND_COMPILE_EXTENDED_STMT`, which causes PHP to emit extra
    /// `ZEND_EXT_STMT` opcodes in every compiled script. Opt in only if your
    /// profiler actually needs per-statement granularity.
    pub fn hook_statements(mut self) -> Self {
        self.hook_statements = true;
        self
    }

    /// Enable `on_fcall_begin` / `on_fcall_end` -- legacy per-call-site hooks.
    ///
    /// Flips `ZEND_COMPILE_EXTENDED_FCALL`, which causes PHP to emit
    /// `ZEND_EXT_FCALL_BEGIN`/`END` opcodes around every call site.
    pub fn hook_fcalls(mut self) -> Self {
        self.hook_fcalls = true;
        self
    }

    /// Consume the builder, register the extension, and return the outer
    /// [`ModuleBuilder`](crate::builders::ModuleBuilder) so further module
    /// config can be chained.
    ///
    /// # Panics
    ///
    /// Panics if a `ZendExtensionHandler` has already been registered on this
    /// module. Each extension may register at most one handler.
    pub fn finish(self) -> crate::builders::ModuleBuilder<'a> {
        register_config(ZendExtensionConfig {
            factory: self.factory,
            hook_op_array: self.hook_op_array,
            hook_statements: self.hook_statements,
            hook_fcalls: self.hook_fcalls,
        });
        self.module
            .expect("ZendExtensionBuilder::finish called on test instance")
    }

    #[cfg(test)]
    pub(crate) fn __for_tests<F, H>(factory: F) -> Self
    where
        F: Fn() -> H + Send + Sync + 'static,
        H: ZendExtensionHandler,
    {
        Self {
            module: None,
            factory: Box::new(move || Box::new(factory())),
            hook_op_array: false,
            hook_statements: false,
            hook_fcalls: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn opts(&self) -> (bool, bool, bool) {
        (self.hook_op_array, self.hook_statements, self.hook_fcalls)
    }
}

pub(crate) fn register_config(config: ZendExtensionConfig) {
    assert!(
        ZEND_EXT_CONFIG.set(config).is_ok(),
        "zend_extension can only be registered once per module",
    );
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
        handler.on_op_array_compiled(op);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_statement_handler(execute_data: *mut ffi::zend_execute_data) {
    if let Some(handler) = get_handler()
        && let Some(ex) = unsafe { execute_data.as_ref() }
    {
        handler.on_statement(ex);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_fcall_begin_handler(execute_data: *mut ffi::zend_execute_data) {
    if let Some(handler) = get_handler()
        && let Some(ex) = unsafe { execute_data.as_ref() }
    {
        handler.on_fcall_begin(ex);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_fcall_end_handler(execute_data: *mut ffi::zend_execute_data) {
    if let Some(handler) = get_handler()
        && let Some(ex) = unsafe { execute_data.as_ref() }
    {
        handler.on_fcall_end(ex);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_op_array_ctor(op_array: *mut ffi::zend_op_array) {
    if let Some(handler) = get_handler()
        && let Some(op) = unsafe { op_array.as_mut() }
    {
        handler.on_op_array_ctor(op);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_op_array_dtor(op_array: *mut ffi::zend_op_array) {
    if let Some(handler) = get_handler()
        && let Some(op) = unsafe { op_array.as_mut() }
    {
        handler.on_op_array_dtor(op);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_message_handler(message: i32, arg: *mut c_void) {
    if let Some(handler) = get_handler() {
        handler.on_message(message, arg);
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_activate() {
    if let Some(cfg) = ZEND_EXT_CONFIG.get() {
        apply_compiler_flags(cfg);
    }
    if let Some(handler) = get_handler() {
        handler.on_activate();
    }
}

/// # Safety
///
/// Called from PHP's C code.
unsafe extern "C" fn ext_deactivate() {
    if let Some(handler) = get_handler() {
        handler.on_deactivate();
    }
}

// ============================================================================
// Registration
// ============================================================================

/// # Safety
///
/// Must be called during MINIT phase only.
pub(crate) unsafe fn zend_extension_startup(name: &str, version: &str) {
    let Some(cfg) = ZEND_EXT_CONFIG.get() else {
        return;
    };

    let _ = ZEND_EXT_INSTANCE.set((cfg.factory)());

    let c_name = CString::new(name).expect("zend extension name must not contain nul bytes");
    let c_version =
        CString::new(version).expect("zend extension version must not contain nul bytes");
    let name_ptr = EXT_NAME.get_or_init(|| c_name).as_ptr();
    let version_ptr = EXT_VERSION.get_or_init(|| c_version).as_ptr();

    let mut ext: ffi::zend_extension = unsafe { std::mem::zeroed() };
    ext.name = name_ptr;
    ext.version = version_ptr;

    // Always-on cold-path hooks.
    ext.activate = Some(ext_activate);
    ext.deactivate = Some(ext_deactivate);
    ext.message_handler = Some(ext_message_handler);
    ext.op_array_ctor = Some(ext_op_array_ctor);
    ext.op_array_dtor = Some(ext_op_array_dtor);

    // Opt-in hot-path hooks.
    if cfg.hook_op_array {
        ext.op_array_handler = Some(ext_op_array_handler);
    }
    if cfg.hook_statements {
        ext.statement_handler = Some(ext_statement_handler);
    }
    if cfg.hook_fcalls {
        ext.fcall_begin_handler = Some(ext_fcall_begin_handler);
        ext.fcall_end_handler = Some(ext_fcall_end_handler);
    }

    unsafe {
        crate::zend::register_extension(&raw mut ext);
    }
    apply_compiler_flags(cfg);
}

fn apply_compiler_flags(cfg: &ZendExtensionConfig) {
    let flags = compile_flags_for(cfg);
    if flags != 0 {
        let mut cg = super::globals::CompilerGlobals::get_mut();
        cg.compiler_options |= flags;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(stmts: bool, fcalls: bool, op_array: bool) -> ZendExtensionConfig {
        ZendExtensionConfig {
            factory: Box::new(|| Box::new(NoopHandler)),
            hook_statements: stmts,
            hook_fcalls: fcalls,
            hook_op_array: op_array,
        }
    }

    struct NoopHandler;
    impl ZendExtensionHandler for NoopHandler {}

    // -- compile_flags_for tests --

    #[test]
    fn compile_flags_for_none_is_zero() {
        assert_eq!(compile_flags_for(&cfg(false, false, false)), 0);
    }

    #[test]
    fn compile_flags_for_statements_only() {
        assert_eq!(
            compile_flags_for(&cfg(true, false, false)),
            ZEND_COMPILE_EXTENDED_STMT,
        );
    }

    #[test]
    fn compile_flags_for_fcalls_only() {
        assert_eq!(
            compile_flags_for(&cfg(false, true, false)),
            ZEND_COMPILE_EXTENDED_FCALL,
        );
    }

    #[test]
    fn compile_flags_for_op_array_only() {
        assert_eq!(
            compile_flags_for(&cfg(false, false, true)),
            ZEND_COMPILE_HANDLE_OP_ARRAY,
        );
    }

    #[test]
    fn compile_flags_for_all_is_or() {
        assert_eq!(
            compile_flags_for(&cfg(true, true, true)),
            ZEND_COMPILE_EXTENDED_STMT | ZEND_COMPILE_EXTENDED_FCALL | ZEND_COMPILE_HANDLE_OP_ARRAY,
        );
    }

    // -- builder state tests --

    #[test]
    fn builder_starts_with_no_hooks() {
        let b = ZendExtensionBuilder::__for_tests(|| NoopHandler);
        assert_eq!(b.opts(), (false, false, false));
    }

    #[test]
    fn builder_hook_statements_flips_only_statements() {
        let b = ZendExtensionBuilder::__for_tests(|| NoopHandler).hook_statements();
        assert_eq!(b.opts(), (false, true, false));
    }

    #[test]
    fn builder_hook_fcalls_flips_only_fcalls() {
        let b = ZendExtensionBuilder::__for_tests(|| NoopHandler).hook_fcalls();
        assert_eq!(b.opts(), (false, false, true));
    }

    #[test]
    fn builder_hook_op_array_compile_flips_only_op_array() {
        let b = ZendExtensionBuilder::__for_tests(|| NoopHandler).hook_op_array_compile();
        assert_eq!(b.opts(), (true, false, false));
    }

    #[test]
    fn builder_hooks_compose() {
        let b = ZendExtensionBuilder::__for_tests(|| NoopHandler)
            .hook_statements()
            .hook_fcalls()
            .hook_op_array_compile();
        assert_eq!(b.opts(), (true, true, true));
    }

    #[test]
    fn builder_hooks_idempotent() {
        let b = ZendExtensionBuilder::__for_tests(|| NoopHandler)
            .hook_statements()
            .hook_statements();
        assert_eq!(b.opts(), (false, true, false));
    }

    #[test]
    fn trait_lifecycle_defaults_are_no_op() {
        // The other six defaults (on_op_array_compiled, on_statement,
        // on_fcall_begin/end, on_op_array_ctor/dtor) require a PHP runtime
        // to construct their arguments; the integration test exercises
        // them end-to-end.
        struct Empty;
        impl ZendExtensionHandler for Empty {}
        let h = Empty;
        h.on_activate();
        h.on_deactivate();
        h.on_message(0, std::ptr::null_mut());
    }
}
