use super::ffi::{
    ext_php_rs_worker_request_shutdown, ext_php_rs_worker_request_startup,
    ext_php_rs_worker_reset_superglobals,
};
use crate::ffi::ZEND_RESULT_CODE_SUCCESS;

/// Errors from the worker request lifecycle.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WorkerError {
    /// `worker_request_startup` returned a non-SUCCESS code.
    #[error("worker request startup failed")]
    StartupFailed,
}

/// Stops the worker request.
///
/// The function removes the pending exception, if there is one. The exception
/// is part of the request that stopped.
///
/// After a request without errors, the function stops only the output layer
/// and the SAPI. The executor stays in memory, thus this sequence is faster
/// than `php_request_shutdown()`.
///
/// After a fatal error or a memory limit error, it is not safe to use the
/// engine again. The function then calls `php_request_shutdown()`, and
/// [`worker_request_startup`] calls `php_request_startup()`.
pub fn worker_request_shutdown() {
    unsafe {
        ext_php_rs_worker_request_shutdown();
    }
}

/// Starts the next worker request, after [`worker_request_shutdown`].
///
/// After a request without errors, the function starts the output layer and
/// the SAPI again. After a full `php_request_shutdown()`, the function calls
/// `php_request_startup()`.
///
/// # Errors
///
/// Returns [`WorkerError::StartupFailed`] if the underlying C call does not
/// return `SUCCESS`.
pub fn worker_request_startup() -> Result<(), WorkerError> {
    let result = unsafe { ext_php_rs_worker_request_startup() };
    if result == ZEND_RESULT_CODE_SUCCESS {
        Ok(())
    } else {
        Err(WorkerError::StartupFailed)
    }
}

/// Force PHP to re-populate all auto-global superglobals
/// (`$_SERVER`, `$_GET`, `$_POST`, etc.).
///
/// Call this after [`worker_request_startup`] to ensure superglobals
/// reflect the new request state.
pub fn worker_reset_superglobals() {
    unsafe {
        ext_php_rs_worker_reset_superglobals();
    }
}
