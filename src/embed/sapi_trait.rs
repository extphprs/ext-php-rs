use crate::builders::SapiBuilder;
use crate::embed::SapiModule;
use crate::embed::context::ServerContext;
use crate::embed::server_vars::ServerVarRegistrar;
use crate::error::Result;
use crate::ffi::{ext_php_rs_sapi_globals, sapi_header_struct, sapi_headers_struct};
use crate::types::Zval;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::NonNull;

/// Safe wrapper around `sapi_headers_struct` providing access to the HTTP
/// response code set by PHP.
///
/// ext-php-rs creates this snapshot and passes it to
/// [`Sapi::send_headers`]. The response code is copied before user code runs,
/// so the safe API does not borrow the Zend structure.
pub struct SapiHeaders {
    http_response_code: i32,
}

impl std::fmt::Debug for SapiHeaders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SapiHeaders")
            .field("http_response_code", &self.http_response_code())
            .finish()
    }
}

impl SapiHeaders {
    /// Copies a `sapi_headers_struct` into an owned snapshot.
    ///
    /// # Safety
    ///
    /// `raw` must be valid and readable for the duration of the callback.
    unsafe fn snapshot(raw: NonNull<sapi_headers_struct>) -> Self {
        // SAFETY: The caller guarantees that `raw` is readable for this copy.
        let http_response_code = unsafe { raw.as_ref().http_response_code };
        Self { http_response_code }
    }

    /// Returns the HTTP response code set by PHP (e.g. 200, 404, 500).
    #[must_use]
    pub fn http_response_code(&self) -> i32 {
        self.http_response_code
    }
}

/// Safe wrapper around `sapi_header_struct` providing access to a single
/// HTTP response header sent by PHP.
///
/// ext-php-rs creates this snapshot and passes it to [`Sapi::send_header`].
/// The header bytes are copied before user code runs, so the safe API does not
/// borrow the Zend structure or its buffer.
pub struct SapiHeader {
    header: Option<Box<[u8]>>,
    header_len: usize,
}

impl std::fmt::Debug for SapiHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SapiHeader")
            .field("header", &self.as_str())
            .finish()
    }
}

impl SapiHeader {
    /// Copies a `sapi_header_struct` into an owned snapshot.
    ///
    /// # Safety
    ///
    /// `raw` must be valid and readable for the duration of the callback. If
    /// its nested header pointer is non-null, it must be readable for exactly
    /// `header_len` bytes during the copy.
    unsafe fn snapshot(raw: NonNull<sapi_header_struct>) -> Self {
        // SAFETY: The caller guarantees that `raw` is readable for this copy.
        let raw = unsafe { raw.as_ref() };
        let header_len = raw.header_len;
        let header = NonNull::new(raw.header.cast::<u8>()).map(|header| {
            if header_len == 0 {
                Box::default()
            } else {
                // SAFETY: The caller guarantees that a non-null nested header
                // pointer is readable for `header_len` bytes during this copy.
                unsafe { std::slice::from_raw_parts(header.as_ptr(), header_len) }.into()
            }
        });

        Self { header, header_len }
    }

    /// Returns the raw header string (e.g. `"Content-Type: text/html"`).
    ///
    /// Returns `None` if the header data is not valid UTF-8.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        if self.header_len == 0 {
            return None;
        }
        std::str::from_utf8(self.header.as_deref()?).ok()
    }

    /// Returns the header parsed as a `(name, value)` pair, splitting on the
    /// first `:`.
    ///
    /// Both name and value are trimmed of whitespace. Returns `None` if the
    /// header is not valid UTF-8 or does not contain `:`.
    #[must_use]
    pub fn as_name_value(&self) -> Option<(&str, &str)> {
        let s = self.as_str()?;
        let (name, value) = s.split_once(':')?;
        Some((name.trim(), value.trim()))
    }

    /// Returns the length of the header string in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.header_len
    }

    /// Returns `true` if the header is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

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
    fn send_headers(_ctx: &mut Self::Context, _headers: &SapiHeaders) -> SendHeadersResult {
        SendHeadersResult::DoSend
    }

    /// Send a single response header.
    fn send_header(_ctx: &mut Self::Context, _header: &SapiHeader) {}

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
    if str.is_null() || str_length == 0 {
        return 0;
    }
    let Some(ctx) = get_server_context::<S>() else {
        return 0;
    };
    let buf = unsafe { std::slice::from_raw_parts(str.cast::<u8>(), str_length) };
    S::ub_write(ctx, buf)
}

extern "C" fn trampoline_log_message<S: Sapi>(message: *const c_char, syslog_type: c_int) {
    if message.is_null() {
        return;
    }
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
    let Some(sapi_headers) = NonNull::new(sapi_headers) else {
        return SendHeadersResult::Failed.into_c_int();
    };
    let Some(ctx) = get_server_context::<S>() else {
        return SendHeadersResult::Failed.into_c_int();
    };
    // SAFETY: PHP provides a readable SAPI headers struct for this callback.
    let headers = unsafe { SapiHeaders::snapshot(sapi_headers) };
    S::send_headers(ctx, &headers).into_c_int()
}

extern "C" fn trampoline_send_header<S: Sapi>(
    header: *mut sapi_header_struct,
    _server_context: *mut c_void,
) {
    let Some(header) = NonNull::new(header) else {
        return;
    };
    if let Some(ctx) = get_server_context::<S>() {
        // SAFETY: PHP provides a readable header struct and nested header
        // buffer for the declared length during this callback.
        let header = unsafe { SapiHeader::snapshot(header) };
        S::send_header(ctx, &header);
    }
}

extern "C" fn trampoline_read_post<S: Sapi>(buffer: *mut c_char, length: usize) -> usize {
    if buffer.is_null() || length == 0 {
        return 0;
    }
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
    if vars.is_null() {
        return;
    }
    let Some(ctx) = get_server_context::<S>() else {
        return;
    };
    let mut registrar = unsafe { ServerVarRegistrar::from_raw(vars) };
    S::register_server_variables(ctx, &mut registrar);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::RequestInfo;

    struct PanicContext;

    impl ServerContext for PanicContext {
        fn init_request_info(&self, _info: &mut RequestInfo) {
            panic!("init_request_info callback must not be invoked");
        }

        fn read_post(&mut self, _buf: &mut [u8]) -> usize {
            panic!("read_post callback must not be invoked");
        }

        fn read_cookies(&self) -> Option<&str> {
            panic!("read_cookies callback must not be invoked");
        }

        fn finish_request(&mut self) -> bool {
            panic!("finish_request callback must not be invoked");
        }

        fn is_request_finished(&self) -> bool {
            panic!("is_request_finished callback must not be invoked");
        }
    }

    struct PanicSapi;

    impl Sapi for PanicSapi {
        type Context = PanicContext;

        fn name() -> &'static str {
            "panic-sapi"
        }

        fn pretty_name() -> &'static str {
            "Panic SAPI"
        }

        fn ub_write(_ctx: &mut Self::Context, _buf: &[u8]) -> usize {
            panic!("ub_write callback must not be invoked");
        }

        fn log_message(_message: &str, _syslog_type: i32) {
            panic!("log_message callback must not be invoked");
        }

        fn send_headers(_ctx: &mut Self::Context, _headers: &SapiHeaders) -> SendHeadersResult {
            panic!("send_headers callback must not be invoked");
        }

        fn send_header(_ctx: &mut Self::Context, _header: &SapiHeader) {
            panic!("send_header callback must not be invoked");
        }
    }

    #[test]
    fn test_sapi_header_valid() {
        let header_bytes = b"Content-Type: text/html";
        let mut raw = sapi_header_struct {
            header: header_bytes.as_ptr().cast_mut().cast::<c_char>(),
            header_len: header_bytes.len(),
        };
        // SAFETY: `raw` and its header buffer are readable for the copy.
        let wrapper = unsafe { SapiHeader::snapshot(NonNull::from(&mut raw)) };

        assert_eq!(wrapper.as_str(), Some("Content-Type: text/html"));
        assert_eq!(wrapper.as_name_value(), Some(("Content-Type", "text/html")));
        assert_eq!(wrapper.len(), 23);
        assert!(!wrapper.is_empty());
    }

    #[test]
    fn test_sapi_header_null_pointer() {
        let mut raw = sapi_header_struct {
            header: std::ptr::null_mut(),
            header_len: 0,
        };
        // SAFETY: `raw` is readable and its nested header pointer is null.
        let wrapper = unsafe { SapiHeader::snapshot(NonNull::from(&mut raw)) };

        assert_eq!(wrapper.as_str(), None);
        assert_eq!(wrapper.as_name_value(), None);
        assert!(wrapper.is_empty());
    }

    #[test]
    fn test_sapi_header_no_colon() {
        let header_bytes = b"InvalidHeader";
        let mut raw = sapi_header_struct {
            header: header_bytes.as_ptr().cast_mut().cast::<c_char>(),
            header_len: header_bytes.len(),
        };
        // SAFETY: `raw` and its header buffer are readable for the copy.
        let wrapper = unsafe { SapiHeader::snapshot(NonNull::from(&mut raw)) };

        assert_eq!(wrapper.as_str(), Some("InvalidHeader"));
        assert_eq!(wrapper.as_name_value(), None);
    }

    #[test]
    fn test_sapi_header_debug_format() {
        let header_bytes = b"X-Custom: value";
        let mut raw = sapi_header_struct {
            header: header_bytes.as_ptr().cast_mut().cast::<c_char>(),
            header_len: header_bytes.len(),
        };
        // SAFETY: `raw` and its header buffer are readable for the copy.
        let wrapper = unsafe { SapiHeader::snapshot(NonNull::from(&mut raw)) };
        let debug = format!("{wrapper:?}");
        assert!(debug.contains("X-Custom: value"));
    }

    #[test]
    fn test_sapi_headers_response_code() {
        // SAFETY: A zeroed C SAPI headers struct has valid null/zero fields.
        let mut raw: sapi_headers_struct = unsafe { std::mem::zeroed() };
        raw.http_response_code = 404;
        // SAFETY: `raw` is readable for the copy.
        let wrapper = unsafe { SapiHeaders::snapshot(NonNull::from(&mut raw)) };

        assert_eq!(wrapper.http_response_code(), 404);
    }

    #[test]
    fn test_sapi_headers_debug_format() {
        // SAFETY: A zeroed C SAPI headers struct has valid null/zero fields.
        let mut raw: sapi_headers_struct = unsafe { std::mem::zeroed() };
        raw.http_response_code = 200;
        // SAFETY: `raw` is readable for the copy.
        let wrapper = unsafe { SapiHeaders::snapshot(NonNull::from(&mut raw)) };
        let debug = format!("{wrapper:?}");
        assert!(debug.contains("200"));
    }

    #[test]
    fn null_header_buffer_preserves_declared_nonzero_length() {
        let mut raw = sapi_header_struct {
            header: std::ptr::null_mut(),
            header_len: 12,
        };
        // SAFETY: `raw` is readable and its nested header pointer is null.
        let wrapper = unsafe { SapiHeader::snapshot(NonNull::from(&mut raw)) };

        assert_eq!(wrapper.as_str(), None);
        assert_eq!(wrapper.as_name_value(), None);
        assert_eq!(wrapper.len(), 12);
        assert!(!wrapper.is_empty());
    }

    #[test]
    fn nonnull_zero_length_header_is_not_dereferenced() {
        let mut raw = sapi_header_struct {
            header: NonNull::<u8>::dangling().as_ptr().cast::<c_char>(),
            header_len: 0,
        };
        // SAFETY: `raw` is readable; a non-null pointer is readable for zero bytes.
        let wrapper = unsafe { SapiHeader::snapshot(NonNull::from(&mut raw)) };

        assert_eq!(wrapper.as_str(), None);
        assert_eq!(wrapper.len(), 0);
        assert!(wrapper.is_empty());
    }

    #[test]
    fn invalid_utf8_header_has_no_string_view() {
        let header_bytes = [0xff, 0xfe];
        let mut raw = sapi_header_struct {
            header: header_bytes.as_ptr().cast_mut().cast::<c_char>(),
            header_len: header_bytes.len(),
        };
        // SAFETY: `raw` and its header buffer are readable for the copy.
        let wrapper = unsafe { SapiHeader::snapshot(NonNull::from(&mut raw)) };

        assert_eq!(wrapper.as_str(), None);
        assert_eq!(wrapper.as_name_value(), None);
        assert_eq!(wrapper.len(), 2);
    }

    #[test]
    fn snapshots_own_copied_source_data() {
        let header = {
            let mut header_bytes = b"X-Owned: original".to_vec();
            let mut raw_header = sapi_header_struct {
                header: header_bytes.as_mut_ptr().cast::<c_char>(),
                header_len: header_bytes.len(),
            };
            // SAFETY: `raw_header` and its header buffer are readable for the copy.
            let header = unsafe { SapiHeader::snapshot(NonNull::from(&mut raw_header)) };

            header_bytes.fill(b'X');
            raw_header.header = std::ptr::null_mut();
            raw_header.header_len = 0;
            assert!(raw_header.header.is_null());
            assert_eq!(raw_header.header_len, 0);
            drop(header_bytes);
            header
        };

        assert_eq!(header.as_str(), Some("X-Owned: original"));
        assert_eq!(header.as_name_value(), Some(("X-Owned", "original")));

        let headers = {
            // SAFETY: A zeroed C SAPI headers struct has valid null/zero fields.
            let mut raw_headers: sapi_headers_struct = unsafe { std::mem::zeroed() };
            raw_headers.http_response_code = 201;
            // SAFETY: `raw_headers` is readable for the copy.
            let headers = unsafe { SapiHeaders::snapshot(NonNull::from(&mut raw_headers)) };
            raw_headers.http_response_code = 500;
            assert_eq!(raw_headers.http_response_code, 500);
            headers
        };

        assert_eq!(headers.http_response_code(), 201);
    }

    #[test]
    fn null_trampoline_inputs_take_fast_paths() {
        assert_eq!(
            trampoline_send_headers::<PanicSapi>(std::ptr::null_mut()),
            SendHeadersResult::Failed.into_c_int()
        );
        trampoline_send_header::<PanicSapi>(std::ptr::null_mut(), std::ptr::null_mut());
    }

    #[test]
    fn send_headers_result_mappings_are_stable() {
        assert_eq!(SendHeadersResult::SentSuccessfully.into_c_int(), 1);
        assert_eq!(SendHeadersResult::DoSend.into_c_int(), 2);
        assert_eq!(SendHeadersResult::Failed.into_c_int(), 3);
    }
}
