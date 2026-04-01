/// Per-request state for a custom SAPI.
///
/// Implementors provide the bridge between an HTTP server's request
/// representation and PHP's SAPI globals. An instance is stored in
/// `SG(server_context)` for the duration of a request.
pub trait ServerContext: Sized + Send {
    /// Populate request info fields before `sapi_activate()`.
    fn init_request_info(&self, info: &mut RequestInfo);

    /// Read a chunk of the POST body into `buf`. Returns bytes read.
    fn read_post(&mut self, buf: &mut [u8]) -> usize;

    /// Return the raw Cookie header value, if any.
    fn read_cookies(&self) -> Option<&str>;

    /// Signal that the response is complete. Returns `false` if already
    /// finished.
    fn finish_request(&mut self) -> bool;

    /// Check whether the response has already been finished.
    fn is_request_finished(&self) -> bool;
}

/// Writable request info for populating `SG(request_info)` before
/// `sapi_activate()`.
///
/// Not to be confused with `SapiRequestInfo` (the read-only type alias for
/// the raw FFI `sapi_request_info` struct defined in `zend::globals`).
#[derive(Debug, Default, Clone)]
pub struct RequestInfo {
    /// HTTP method (GET, POST, etc.)
    pub request_method: Option<String>,
    /// Query string (after `?`)
    pub query_string: Option<String>,
    /// Full request URI
    pub request_uri: Option<String>,
    /// Filesystem path to the script
    pub path_translated: Option<String>,
    /// Content-Type header value
    pub content_type: Option<String>,
    /// Content-Length header value
    pub content_length: i64,
    /// HTTP protocol version: 1000 = HTTP/1.0, 1100 = HTTP/1.1, 2000 = HTTP/2
    pub proto_num: u16,
    /// HTTP Basic auth username
    pub auth_user: Option<String>,
    /// HTTP Basic auth password
    pub auth_password: Option<String>,
}
