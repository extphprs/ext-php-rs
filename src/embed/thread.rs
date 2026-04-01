use super::ffi::{ext_php_rs_sapi_per_thread_init, ext_php_rs_sapi_per_thread_shutdown};

/// RAII guard for PHP ZTS thread-local state.
///
/// Initializes PHP thread-local storage on creation and cleans it up on drop.
/// Must be created on a dedicated OS thread after the main SAPI has been
/// started with `sapi_startup()` + `php_module_startup()`.
///
/// # Safety
///
/// The guard must not outlive the SAPI module. Drop all guards before calling
/// `php_module_shutdown()`.
///
/// # Examples
///
/// ```rust,no_run
/// use ext_php_rs::embed::PhpThreadGuard;
///
/// std::thread::spawn(|| {
///     let _guard = PhpThreadGuard::new();
///     // ... execute PHP requests on this thread ...
/// });
/// ```
#[must_use]
pub struct PhpThreadGuard {
    _private: (),
}

impl PhpThreadGuard {
    /// Initialize PHP thread-local state for the current OS thread.
    pub fn new() -> Self {
        unsafe {
            ext_php_rs_sapi_per_thread_init();
        }
        Self { _private: () }
    }
}

impl Default for PhpThreadGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PhpThreadGuard {
    fn drop(&mut self) {
        unsafe {
            ext_php_rs_sapi_per_thread_shutdown();
        }
    }
}
