use crate::embed::ext_php_rs_php_error;
use crate::ffi::{
    gid_t, php_default_input_filter, php_default_post_reader, php_default_treat_data,
    sapi_header_struct, sapi_headers_struct, uid_t,
};
use crate::types::Zval;
use crate::{embed::SapiModule, error::Result};

use std::{
    ffi::{CString, c_char, c_int, c_void},
    ptr,
};

/// Builder for `SapiModule`s
///
/// # Example
///
/// ```rust,no_run
/// use std::ffi::{c_char, c_int, c_void};
/// use ext_php_rs::{
///     builders::SapiBuilder,
///     ffi::sapi_header_struct
/// };
///
/// #[unsafe(no_mangle)]
/// pub extern "C" fn ub_write(str: *const i8, str_length: usize) -> usize {
///     println!("PHP wrote: {:?}", str);
///     str_length
/// }
///
/// #[unsafe(no_mangle)]
/// pub extern "C" fn send_header(header: *mut sapi_header_struct, server_context: *mut c_void) {
///     println!("PHP sent a header: {:?}", header);
/// }
///
/// let sapi = SapiBuilder::new("ext_php_rs", "Ext PHP RS")
///   .ub_write_function(ub_write)
///   .send_header_function(send_header)
///   .build();
/// ```
#[must_use]
pub struct SapiBuilder {
    name: String,
    pretty_name: String,
    module: SapiModule,
    executable_location: Option<String>,
    php_ini_path_override: Option<String>,
    ini_entries: Option<String>,
}

impl SapiBuilder {
    /// Creates a new [`SapiBuilder`] instance
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the SAPI module.
    pub fn new<T: Into<String>, U: Into<String>>(name: T, pretty_name: U) -> Self {
        Self {
            name: name.into(),
            pretty_name: pretty_name.into(),
            module: SapiModule {
                name: ptr::null_mut(),
                pretty_name: ptr::null_mut(),
                startup: None,
                shutdown: None,
                activate: None,
                deactivate: None,
                ub_write: None,
                flush: None,
                get_stat: None,
                getenv: None,
                sapi_error: None,
                header_handler: None,
                send_headers: None,
                send_header: None,
                read_post: None,
                read_cookies: None,
                register_server_variables: None,
                log_message: None,
                get_request_time: None,
                terminate_process: None,
                php_ini_path_override: ptr::null_mut(),
                default_post_reader: None,
                treat_data: None,
                executable_location: ptr::null_mut(),
                php_ini_ignore: 0,
                php_ini_ignore_cwd: 0,
                get_fd: None,
                force_http_10: None,
                get_target_uid: None,
                get_target_gid: None,
                input_filter: None,
                ini_defaults: None,
                phpinfo_as_text: 0,
                ini_entries: ptr::null_mut(),
                additional_functions: ptr::null(),
                input_filter_init: None,
                #[cfg(php85)]
                pre_request_init: None,
            },
            executable_location: None,
            php_ini_path_override: None,
            ini_entries: None,
        }
    }

    /// Sets the startup function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called on startup.
    pub fn startup_function(mut self, func: SapiStartupFunc) -> Self {
        self.module.startup = Some(func);
        self
    }

    /// Sets the shutdown function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called on shutdown.
    pub fn shutdown_function(mut self, func: SapiShutdownFunc) -> Self {
        self.module.shutdown = Some(func);
        self
    }

    /// Sets the activate function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called on activation.
    pub fn activate_function(mut self, func: SapiActivateFunc) -> Self {
        self.module.activate = Some(func);
        self
    }

    /// Sets the deactivate function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called on deactivation.
    pub fn deactivate_function(mut self, func: SapiDeactivateFunc) -> Self {
        self.module.deactivate = Some(func);
        self
    }

    /// Sets the `ub_write` function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called on write.
    pub fn ub_write_function(mut self, func: SapiUbWriteFunc) -> Self {
        self.module.ub_write = Some(func);
        self
    }

    /// Set the flush function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called on flush.
    pub fn flush_function(mut self, func: SapiFlushFunc) -> Self {
        self.module.flush = Some(func);
        self
    }

    /// Sets the get env function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called when PHP gets an environment variable.
    pub fn getenv_function(mut self, func: SapiGetEnvFunc) -> Self {
        self.module.getenv = Some(func);
        self
    }

    /// Sets the sapi error function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called when PHP encounters an error.
    pub fn sapi_error_function(mut self, func: SapiErrorFunc) -> Self {
        self.module.sapi_error = Some(func);
        self
    }

    /// Sets the send header function for this SAPI
    ///
    /// # Arguments
    ///
    /// * `func` - The function to be called on shutdown.
    pub fn send_header_function(mut self, func: SapiSendHeaderFunc) -> Self {
        self.module.send_header = Some(func);
        self
    }

    /// Sets the send headers function for this SAPI
    ///
    /// This function is called once when all headers are finalized and ready to send.
    ///
    /// # Arguments
    ///
    /// * `func` - The function to be called when headers are ready.
    pub fn send_headers_function(mut self, func: SapiSendHeadersFunc) -> Self {
        self.module.send_headers = Some(func);
        self
    }

    /// Sets the read post function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called when PHP reads the POST data.
    pub fn read_post_function(mut self, func: SapiReadPostFunc) -> Self {
        self.module.read_post = Some(func);
        self
    }

    /// Sets the read cookies function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called when PHP reads the cookies.
    pub fn read_cookies_function(mut self, func: SapiReadCookiesFunc) -> Self {
        self.module.read_cookies = Some(func);
        self
    }

    /// Sets the register server variables function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called when PHP registers server variables.
    pub fn register_server_variables_function(
        mut self,
        func: SapiRegisterServerVariablesFunc,
    ) -> Self {
        self.module.register_server_variables = Some(func);
        self
    }

    /// Sets the log message function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called when PHP logs a message.
    pub fn log_message_function(mut self, func: SapiLogMessageFunc) -> Self {
        self.module.log_message = Some(func);
        self
    }

    /// Sets the request time function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called when PHP gets the request time.
    pub fn get_request_time_function(mut self, func: SapiRequestTimeFunc) -> Self {
        self.module.get_request_time = Some(func);
        self
    }

    /// Sets the terminate process function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called when PHP terminates the process.
    pub fn terminate_process_function(mut self, func: SapiTerminateProcessFunc) -> Self {
        self.module.terminate_process = Some(func);
        self
    }

    /// Sets the get uid function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called when PHP gets the uid.
    pub fn get_target_uid_function(mut self, func: SapiGetUidFunc) -> Self {
        self.module.get_target_uid = Some(func);
        self
    }

    /// Sets the get gid function for this SAPI
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called when PHP gets the gid.
    pub fn get_target_gid_function(mut self, func: SapiGetGidFunc) -> Self {
        self.module.get_target_gid = Some(func);
        self
    }

    /// Sets the pre-request init function for this SAPI
    ///
    /// This function is called before request activation and before POST data is read.
    /// It is typically used for .user.ini processing.
    ///
    /// # Parameters
    ///
    /// * `func` - The function to be called before request initialization.
    #[cfg(php85)]
    pub fn pre_request_init_function(mut self, func: SapiPreRequestInitFunc) -> Self {
        self.module.pre_request_init = Some(func);
        self
    }

    /// Set the `ini_entries` for this SAPI
    ///
    /// # Parameters
    ///
    /// * `entries` - A pointer to the ini entries.
    pub fn ini_entries<E: Into<String>>(mut self, entries: E) -> Self {
        self.ini_entries = Some(entries.into());
        self
    }

    /// Sets the php ini path override for this SAPI
    ///
    /// # Parameters
    ///
    /// * `path` - The path to the php ini file.
    pub fn php_ini_path_override<S: Into<String>>(mut self, path: S) -> Self {
        self.php_ini_path_override = Some(path.into());
        self
    }

    /// Sets the php ini ignore for this SAPI
    ///
    /// # Parameters
    ///
    /// * `ignore` - The value to set php ini ignore to.
    pub fn php_ini_ignore(mut self, ignore: i32) -> Self {
        self.module.php_ini_ignore = ignore as c_int;
        self
    }

    /// Sets the php ini ignore cwd for this SAPI
    ///
    /// # Parameters
    ///
    /// * `ignore` - The value to set php ini ignore cwd to.
    pub fn php_ini_ignore_cwd(mut self, ignore: i32) -> Self {
        self.module.php_ini_ignore_cwd = ignore as c_int;
        self
    }

    /// Sets the executable location for this SAPI
    ///
    /// # Parameters
    ///
    /// * `location` - The location of the executable.
    pub fn executable_location<S: Into<String>>(mut self, location: S) -> Self {
        self.executable_location = Some(location.into());
        self
    }

    /// Builds the extension and returns a `SapiModule`.
    ///
    /// Returns a result containing the sapi module if successful.
    ///
    /// # Errors
    ///
    /// * If name or property name contain null bytes
    pub fn build(mut self) -> Result<SapiModule> {
        self.module.name = CString::new(self.name)?.into_raw();
        self.module.pretty_name = CString::new(self.pretty_name)?.into_raw();

        if let Some(path) = self.executable_location {
            self.module.executable_location = CString::new(path)?.into_raw();
        }

        if let Some(entries) = self.ini_entries {
            self.module.ini_entries = CString::new(entries)?.into_raw();
        }

        if let Some(path) = self.php_ini_path_override {
            self.module.php_ini_path_override = CString::new(path)?.into_raw();
        }

        if self.module.send_header.is_none() {
            self.module.send_header = Some(dummy_send_header);
        }

        if self.module.sapi_error.is_none() {
            self.module.sapi_error = Some(ext_php_rs_php_error);
        }

        if self.module.default_post_reader.is_none() {
            self.module.default_post_reader = Some(php_default_post_reader);
        }

        if self.module.treat_data.is_none() {
            self.module.treat_data = Some(php_default_treat_data);
        }

        if self.module.input_filter.is_none() {
            self.module.input_filter = Some(php_default_input_filter);
        }

        Ok(self.module)
    }
}

/// A function to be called when PHP starts the SAPI
pub type SapiStartupFunc = extern "C" fn(sapi: *mut SapiModule) -> c_int;

/// A function to be called when PHP stops the SAPI
pub type SapiShutdownFunc = extern "C" fn(sapi: *mut SapiModule) -> c_int;

/// A function to be called when PHP activates the SAPI
pub type SapiActivateFunc = extern "C" fn() -> c_int;

/// A function to be called when PHP deactivates the SAPI
pub type SapiDeactivateFunc = extern "C" fn() -> c_int;

/// A function to be called when PHP send a header
pub type SapiSendHeaderFunc =
    extern "C" fn(header: *mut sapi_header_struct, server_context: *mut c_void);

/// A function to be called when PHP finalizes all headers
pub type SapiSendHeadersFunc = extern "C" fn(sapi_headers: *mut sapi_headers_struct) -> c_int;

/// A function to be called when PHP write to the output buffer
pub type SapiUbWriteFunc = extern "C" fn(str: *const c_char, str_length: usize) -> usize;

/// A function to be called when PHP flush the output buffer
pub type SapiFlushFunc = extern "C" fn(*mut c_void);

/// A function to be called when PHP gets an environment variable
pub type SapiGetEnvFunc = extern "C" fn(name: *const c_char, name_length: usize) -> *mut c_char;

/// A function to be called when PHP encounters an error
pub type SapiErrorFunc = extern "C" fn(type_: c_int, error_msg: *const c_char, args: ...);

/// A function to be called when PHP read the POST data
pub type SapiReadPostFunc = extern "C" fn(buffer: *mut c_char, length: usize) -> usize;

/// A function to be called when PHP read the cookies
pub type SapiReadCookiesFunc = extern "C" fn() -> *mut c_char;

/// A function to be called when PHP register server variables
pub type SapiRegisterServerVariablesFunc = extern "C" fn(vars: *mut Zval);

/// A function to be called when PHP logs a message
pub type SapiLogMessageFunc = extern "C" fn(message: *const c_char, syslog_type_int: c_int);

/// A function to be called when PHP gets the request time
pub type SapiRequestTimeFunc = extern "C" fn(time: *mut f64) -> c_int;

/// A function to be called when PHP terminates the process
pub type SapiTerminateProcessFunc = extern "C" fn();

/// A function to be called when PHP gets the uid
pub type SapiGetUidFunc = extern "C" fn(uid: *mut uid_t) -> c_int;

/// A function to be called when PHP gets the gid
pub type SapiGetGidFunc = extern "C" fn(gid: *mut gid_t) -> c_int;

/// A function to be called before request activation (used for .user.ini processing)
#[cfg(php85)]
pub type SapiPreRequestInitFunc = extern "C" fn() -> c_int;

extern "C" fn dummy_send_header(_header: *mut sapi_header_struct, _server_context: *mut c_void) {}

#[cfg(test)]
mod test {
    use super::*;
    use std::ffi::CStr;

    extern "C" fn test_startup(_sapi: *mut SapiModule) -> c_int {
        0
    }
    extern "C" fn test_shutdown(_sapi: *mut SapiModule) -> c_int {
        0
    }
    extern "C" fn test_activate() -> c_int {
        0
    }
    extern "C" fn test_deactivate() -> c_int {
        0
    }
    extern "C" fn test_ub_write(_str: *const c_char, _str_length: usize) -> usize {
        0
    }
    extern "C" fn test_flush(_server_context: *mut c_void) {}
    extern "C" fn test_getenv(_name: *const c_char, _name_length: usize) -> *mut c_char {
        ptr::null_mut()
    }
    // Note: C-variadic functions are unstable in Rust, so we can't test this properly
    // extern "C" fn test_sapi_error(_type: c_int, _error_msg: *const c_char, _args: ...) {}
    extern "C" fn test_send_header(_header: *mut sapi_header_struct, _server_context: *mut c_void) {
    }
    extern "C" fn test_send_headers(_sapi_headers: *mut sapi_headers_struct) -> c_int {
        0
    }
    extern "C" fn test_read_post(_buffer: *mut c_char, _length: usize) -> usize {
        0
    }
    extern "C" fn test_read_cookies() -> *mut c_char {
        ptr::null_mut()
    }
    extern "C" fn test_register_server_variables(_vars: *mut Zval) {}
    extern "C" fn test_log_message(_message: *const c_char, _syslog_type_int: c_int) {}
    extern "C" fn test_get_request_time(_time: *mut f64) -> c_int {
        0
    }
    extern "C" fn test_terminate_process() {}
    extern "C" fn test_get_target_uid(_uid: *mut uid_t) -> c_int {
        0
    }
    extern "C" fn test_get_target_gid(_gid: *mut gid_t) -> c_int {
        0
    }
    #[cfg(php85)]
    extern "C" fn test_pre_request_init() -> c_int {
        0
    }

    #[test]
    fn test_basic_sapi_builder() {
        let sapi = SapiBuilder::new("test_sapi", "Test SAPI")
            .build()
            .expect("should build sapi module");

        assert_eq!(
            unsafe { CStr::from_ptr(sapi.name) }
                .to_str()
                .expect("should convert CStr to str"),
            "test_sapi"
        );
        assert_eq!(
            unsafe { CStr::from_ptr(sapi.pretty_name) }
                .to_str()
                .expect("should convert CStr to str"),
            "Test SAPI"
        );
    }

    #[test]
    fn test_startup_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .startup_function(test_startup)
            .build()
            .expect("should build sapi module");

        assert!(sapi.startup.is_some());
        assert_eq!(
            sapi.startup.expect("should have startup function") as usize,
            test_startup as usize
        );
    }

    #[test]
    fn test_shutdown_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .shutdown_function(test_shutdown)
            .build()
            .expect("should build sapi module");

        assert!(sapi.shutdown.is_some());
        assert_eq!(
            sapi.shutdown.expect("should have shutdown function") as usize,
            test_shutdown as usize
        );
    }

    #[test]
    fn test_activate_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .activate_function(test_activate)
            .build()
            .expect("should build sapi module");

        assert!(sapi.activate.is_some());
        assert_eq!(
            sapi.activate.expect("should have activate function") as usize,
            test_activate as usize
        );
    }

    #[test]
    fn test_deactivate_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .deactivate_function(test_deactivate)
            .build()
            .expect("should build sapi module");

        assert!(sapi.deactivate.is_some());
        assert_eq!(
            sapi.deactivate.expect("should have deactivate function") as usize,
            test_deactivate as usize
        );
    }

    #[test]
    fn test_ub_write_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .ub_write_function(test_ub_write)
            .build()
            .expect("should build sapi module");

        assert!(sapi.ub_write.is_some());
        assert_eq!(
            sapi.ub_write.expect("should have ub_write function") as usize,
            test_ub_write as usize
        );
    }

    #[test]
    fn test_flush_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .flush_function(test_flush)
            .build()
            .expect("should build sapi module");

        assert!(sapi.flush.is_some());
        assert_eq!(
            sapi.flush.expect("should have flush function") as usize,
            test_flush as usize
        );
    }

    #[test]
    fn test_getenv_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .getenv_function(test_getenv)
            .build()
            .expect("should build sapi module");

        assert!(sapi.getenv.is_some());
        assert_eq!(
            sapi.getenv.expect("should have getenv function") as usize,
            test_getenv as usize
        );
    }

    // Note: Cannot test sapi_error_function because C-variadic functions are unstable in Rust
    // The sapi_error field accepts a function with variadic arguments which cannot be
    // created in stable Rust. However, the builder method itself works correctly.

    #[test]
    fn test_send_header_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .send_header_function(test_send_header)
            .build()
            .expect("should build sapi module");

        assert!(sapi.send_header.is_some());
        assert_eq!(
            sapi.send_header.expect("should have send_header function") as usize,
            test_send_header as usize
        );
    }

    #[test]
    fn test_send_headers_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .send_headers_function(test_send_headers)
            .build()
            .expect("should build sapi module");

        assert!(sapi.send_headers.is_some());
        assert_eq!(
            sapi.send_headers
                .expect("should have send_headers function") as usize,
            test_send_headers as usize
        );
    }

    #[test]
    fn test_read_post_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .read_post_function(test_read_post)
            .build()
            .expect("should build sapi module");

        assert!(sapi.read_post.is_some());
        assert_eq!(
            sapi.read_post.expect("should have read_post function") as usize,
            test_read_post as usize
        );
    }

    #[test]
    fn test_read_cookies_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .read_cookies_function(test_read_cookies)
            .build()
            .expect("should build sapi module");

        assert!(sapi.read_cookies.is_some());
        assert_eq!(
            sapi.read_cookies
                .expect("should have read_cookies function") as usize,
            test_read_cookies as usize
        );
    }

    #[test]
    fn test_register_server_variables_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .register_server_variables_function(test_register_server_variables)
            .build()
            .expect("should build sapi module");

        assert!(sapi.register_server_variables.is_some());
        assert_eq!(
            sapi.register_server_variables
                .expect("should have register_server_variables function") as usize,
            test_register_server_variables as usize
        );
    }

    #[test]
    fn test_log_message_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .log_message_function(test_log_message)
            .build()
            .expect("should build sapi module");

        assert!(sapi.log_message.is_some());
        assert_eq!(
            sapi.log_message.expect("should have log_message function") as usize,
            test_log_message as usize
        );
    }

    #[test]
    fn test_get_request_time_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .get_request_time_function(test_get_request_time)
            .build()
            .expect("should build sapi module");

        assert!(sapi.get_request_time.is_some());
        assert_eq!(
            sapi.get_request_time
                .expect("should have request_time function") as usize,
            test_get_request_time as usize
        );
    }

    #[test]
    fn test_terminate_process_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .terminate_process_function(test_terminate_process)
            .build()
            .expect("should build sapi module");

        assert!(sapi.terminate_process.is_some());
        assert_eq!(
            sapi.terminate_process
                .expect("should have terminate_process function") as usize,
            test_terminate_process as usize
        );
    }

    #[test]
    fn test_get_target_uid_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .get_target_uid_function(test_get_target_uid)
            .build()
            .expect("should build sapi module");

        assert!(sapi.get_target_uid.is_some());
        assert_eq!(
            sapi.get_target_uid
                .expect("should have get_target_uid function") as usize,
            test_get_target_uid as usize
        );
    }

    #[test]
    fn test_get_target_gid_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .get_target_gid_function(test_get_target_gid)
            .build()
            .expect("should build sapi module");

        assert!(sapi.get_target_gid.is_some());
        assert_eq!(
            sapi.get_target_gid
                .expect("should have get_target_gid function") as usize,
            test_get_target_gid as usize
        );
    }

    #[cfg(php85)]
    #[test]
    fn test_pre_request_init_function() {
        let sapi = SapiBuilder::new("test", "Test")
            .pre_request_init_function(test_pre_request_init)
            .build()
            .expect("should build sapi module");

        assert!(sapi.pre_request_init.is_some());
        assert_eq!(
            sapi.pre_request_init
                .expect("should have pre_request_init function") as usize,
            test_pre_request_init as usize
        );
    }

    #[cfg(php82)]
    #[test]
    fn test_sapi_ini_entries() {
        let mut ini = crate::builders::IniBuilder::new();
        ini.define("foo=bar").expect("should define ini entry");
        ini.quoted("memory_limit", "128M")
            .expect("should add quoted ini entry");

        let sapi = SapiBuilder::new("test", "Test")
            .ini_entries(ini)
            .build()
            .expect("should build sapi module");

        assert!(!sapi.ini_entries.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(sapi.ini_entries) },
            c"foo=bar\nmemory_limit=\"128M\"\n"
        );
    }

    #[test]
    fn test_php_ini_path_override() {
        let sapi = SapiBuilder::new("test", "Test")
            .php_ini_path_override("/custom/path/php.ini")
            .build()
            .expect("should build sapi module");

        assert!(!sapi.php_ini_path_override.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(sapi.php_ini_path_override) },
            c"/custom/path/php.ini"
        );
    }

    #[test]
    fn test_php_ini_ignore() {
        let sapi = SapiBuilder::new("test", "Test")
            .php_ini_ignore(1)
            .build()
            .expect("should build sapi module");

        assert_eq!(sapi.php_ini_ignore, 1);
    }

    #[test]
    fn test_php_ini_ignore_cwd() {
        let sapi = SapiBuilder::new("test", "Test")
            .php_ini_ignore_cwd(1)
            .build()
            .expect("should build sapi module");

        assert_eq!(sapi.php_ini_ignore_cwd, 1);
    }

    #[test]
    fn test_executable_location() {
        let sapi = SapiBuilder::new("test", "Test")
            .executable_location("/usr/bin/php")
            .build()
            .expect("should build sapi module");

        assert!(!sapi.executable_location.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(sapi.executable_location) },
            c"/usr/bin/php"
        );
    }

    #[test]
    fn test_default_functions_set() {
        let sapi = SapiBuilder::new("test", "Test")
            .build()
            .expect("should build sapi module");

        // Test that default functions are set
        assert!(sapi.send_header.is_some());
        assert!(sapi.sapi_error.is_some());
        assert!(sapi.default_post_reader.is_some());
        assert!(sapi.treat_data.is_some());
        assert!(sapi.input_filter.is_some());
    }

    #[test]
    fn test_chained_builder() {
        let sapi = SapiBuilder::new("chained", "Chained SAPI")
            .startup_function(test_startup)
            .shutdown_function(test_shutdown)
            .activate_function(test_activate)
            .deactivate_function(test_deactivate)
            .ub_write_function(test_ub_write)
            .flush_function(test_flush)
            .php_ini_path_override("/test/php.ini")
            .php_ini_ignore(1)
            .executable_location("/test/php")
            .build()
            .expect("should build sapi module");

        assert_eq!(unsafe { CStr::from_ptr(sapi.name) }, c"chained");
        assert_eq!(unsafe { CStr::from_ptr(sapi.pretty_name) }, c"Chained SAPI");
        assert!(sapi.startup.is_some());
        assert!(sapi.shutdown.is_some());
        assert!(sapi.activate.is_some());
        assert!(sapi.deactivate.is_some());
        assert!(sapi.ub_write.is_some());
        assert!(sapi.flush.is_some());
        assert_eq!(sapi.php_ini_ignore, 1);
        assert!(!sapi.php_ini_path_override.is_null());
        assert!(!sapi.executable_location.is_null());
    }
}
