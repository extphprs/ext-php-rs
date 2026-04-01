use crate::builders::SapiBuilder;
use crate::embed::SapiModule;
use crate::embed::context::ServerContext;
use crate::embed::server_vars::ServerVarRegistrar;
use crate::error::Result;
use crate::ffi::{ext_php_rs_sapi_globals, sapi_header_struct, sapi_headers_struct};
use crate::types::Zval;
use std::ffi::{c_char, c_int, c_void};

/// Result type for the `send_headers` SAPI callback.
#[non_exhaustive]
pub enum SendHeadersResult {
    /// SAPI handled all headers. PHP will not call `send_header` per header.
    SentSuccessfully,
    /// PHP should iterate headers and call `send_header` for each one.
    DoSend,
    /// Header sending failed.
    Failed,
}

impl SendHeadersResult {
    fn into_c_int(self) -> c_int {
        match self {
            Self::SentSuccessfully => 1,
            Self::DoSend => 2,
            Self::Failed => 3,
        }
    }
}

/// High-level trait for implementing a custom PHP SAPI in safe Rust.
///
/// Generates `extern "C"` trampoline functions that retrieve `Self::Context`
/// from `SG(server_context)` and dispatch to safe trait methods.
///
/// # Examples
///
/// ```rust,no_run
/// use ext_php_rs::embed::{Sapi, ServerContext, RequestInfo, ServerVarRegistrar};
///
/// struct MySapi;
/// struct MyCtx;
///
/// impl ServerContext for MyCtx {
///     fn init_request_info(&self, _info: &mut RequestInfo) {}
///     fn read_post(&mut self, _buf: &mut [u8]) -> usize { 0 }
///     fn read_cookies(&self) -> Option<&str> { None }
///     fn finish_request(&mut self) -> bool { true }
///     fn is_request_finished(&self) -> bool { true }
/// }
///
/// impl Sapi for MySapi {
///     type Context = MyCtx;
///     fn name() -> &'static str { "my-sapi" }
///     fn pretty_name() -> &'static str { "My SAPI" }
///     fn ub_write(_ctx: &mut MyCtx, buf: &[u8]) -> usize { buf.len() }
///     fn log_message(msg: &str, _: i32) { eprintln!("{msg}"); }
/// }
/// ```
pub trait Sapi: Send + Sync + 'static {
    /// Per-request context type.
    type Context: ServerContext;

    /// SAPI identifier (e.g. "ferron-php").
    fn name() -> &'static str;

    /// Human-readable SAPI name (e.g. "Ferron PHP Module").
    fn pretty_name() -> &'static str;

    /// Write output. Called by PHP's `echo`, `print`, etc.
    fn ub_write(ctx: &mut Self::Context, buf: &[u8]) -> usize;

    /// Log a message from PHP.
    fn log_message(message: &str, syslog_type: i32);

    /// Flush output buffer.
    fn flush(_ctx: &mut Self::Context) {}

    /// Send all response headers at once.
    fn send_headers(
        _ctx: &mut Self::Context,
        _headers: *mut sapi_headers_struct,
    ) -> SendHeadersResult {
        SendHeadersResult::DoSend
    }

    /// Send a single response header.
    fn send_header(_ctx: &mut Self::Context, _header: *mut sapi_header_struct) {}

    /// Read POST body chunk. Delegates to `ServerContext::read_post` by default.
    fn read_post(ctx: &mut Self::Context, buf: &mut [u8]) -> usize {
        ctx.read_post(buf)
    }

    /// Read cookie header. Delegates to `ServerContext::read_cookies` by
    /// default.
    fn read_cookies(ctx: &mut Self::Context) -> Option<String> {
        ctx.read_cookies().map(String::from)
    }

    /// Register `$_SERVER` variables.
    fn register_server_variables(_ctx: &mut Self::Context, _registrar: &mut ServerVarRegistrar) {}

    /// Build a [`SapiModule`] from this trait implementation.
    ///
    /// # Errors
    ///
    /// Returns an error if the SAPI name or pretty name contain null bytes.
    fn build_module() -> Result<SapiModule>
    where
        Self: Sized,
    {
        SapiBuilder::new(Self::name(), Self::pretty_name())
            .ub_write_function(trampoline_ub_write::<Self>)
            .log_message_function(trampoline_log_message::<Self>)
            .flush_function(trampoline_flush::<Self>)
            .send_headers_function(trampoline_send_headers::<Self>)
            .send_header_function(trampoline_send_header::<Self>)
            .read_post_function(trampoline_read_post::<Self>)
            .read_cookies_function(trampoline_read_cookies::<Self>)
            .register_server_variables_function(trampoline_register_server_variables::<Self>)
            .build()
    }
}

fn get_server_context<S: Sapi>() -> Option<&'static mut S::Context> {
    let globals = unsafe { &*ext_php_rs_sapi_globals() };
    let ctx_ptr = globals.server_context;
    if ctx_ptr.is_null() {
        return None;
    }
    Some(unsafe { &mut *ctx_ptr.cast::<S::Context>() })
}

extern "C" fn trampoline_ub_write<S: Sapi>(str: *const c_char, str_length: usize) -> usize {
    let Some(ctx) = get_server_context::<S>() else {
        return 0;
    };
    let buf = unsafe { std::slice::from_raw_parts(str.cast::<u8>(), str_length) };
    S::ub_write(ctx, buf)
}

extern "C" fn trampoline_log_message<S: Sapi>(message: *const c_char, syslog_type: c_int) {
    let msg = unsafe { std::ffi::CStr::from_ptr(message) };
    let msg_str = msg.to_string_lossy();
    S::log_message(&msg_str, syslog_type);
}

extern "C" fn trampoline_flush<S: Sapi>(server_context: *mut c_void) {
    let _ = server_context;
    if let Some(ctx) = get_server_context::<S>() {
        S::flush(ctx);
    }
}

extern "C" fn trampoline_send_headers<S: Sapi>(sapi_headers: *mut sapi_headers_struct) -> c_int {
    let Some(ctx) = get_server_context::<S>() else {
        return SendHeadersResult::Failed.into_c_int();
    };
    S::send_headers(ctx, sapi_headers).into_c_int()
}

extern "C" fn trampoline_send_header<S: Sapi>(
    header: *mut sapi_header_struct,
    _server_context: *mut c_void,
) {
    if let Some(ctx) = get_server_context::<S>() {
        S::send_header(ctx, header);
    }
}

extern "C" fn trampoline_read_post<S: Sapi>(buffer: *mut c_char, length: usize) -> usize {
    let Some(ctx) = get_server_context::<S>() else {
        return 0;
    };
    let buf = unsafe { std::slice::from_raw_parts_mut(buffer.cast::<u8>(), length) };
    S::read_post(ctx, buf)
}

extern "C" fn trampoline_read_cookies<S: Sapi>() -> *mut c_char {
    let Some(ctx) = get_server_context::<S>() else {
        return std::ptr::null_mut();
    };
    match S::read_cookies(ctx) {
        Some(cookies) => match std::ffi::CString::new(cookies) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

extern "C" fn trampoline_register_server_variables<S: Sapi>(vars: *mut Zval) {
    let Some(ctx) = get_server_context::<S>() else {
        return;
    };
    let mut registrar = unsafe { ServerVarRegistrar::from_raw(vars) };
    S::register_server_variables(ctx, &mut registrar);
}
