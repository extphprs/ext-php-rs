use crate::ffi::php_register_variable_safe;
use crate::types::Zval;
use std::ffi::CString;

/// Safe wrapper around PHP's `php_register_variable_safe()` for populating
/// `$_SERVER` and other `track_vars` arrays.
///
/// # Examples
///
/// ```rust,no_run
/// use ext_php_rs::embed::ServerVarRegistrar;
/// use ext_php_rs::types::Zval;
///
/// unsafe extern "C" fn register_vars(vars: *mut Zval) {
///     let mut registrar = unsafe { ServerVarRegistrar::from_raw(vars) };
///     registrar.register("SERVER_SOFTWARE", "my-server/1.0");
/// }
/// ```
pub struct ServerVarRegistrar {
    track_vars: *mut Zval,
}

impl ServerVarRegistrar {
    /// Create a registrar from the raw `track_vars_array` pointer
    /// passed to the `register_server_variables` SAPI callback.
    ///
    /// # Safety
    ///
    /// `track_vars` must be a valid pointer to the zval array that PHP passes
    /// to the `register_server_variables` callback.
    #[must_use]
    pub unsafe fn from_raw(track_vars: *mut Zval) -> Self {
        Self { track_vars }
    }

    /// Register a server variable with a string value.
    pub fn register(&mut self, name: &str, value: &str) {
        let Ok(c_name) = CString::new(name) else {
            return;
        };
        unsafe {
            php_register_variable_safe(
                c_name.as_ptr().cast_mut(),
                value.as_ptr().cast(),
                value.len(),
                self.track_vars.cast(),
            );
        }
    }
}
