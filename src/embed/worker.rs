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

/// Run the lightweight request shutdown sequence (output + SAPI teardown).
///
/// This is cheaper than `php_request_shutdown()` because it skips
/// executor destruction, making it suitable for worker-mode recycling.
pub fn worker_request_shutdown() {
    unsafe {
        ext_php_rs_worker_request_shutdown();
    }
}

/// Run the lightweight request startup sequence (output + SAPI activation).
///
/// This re-activates the output layer and SAPI globals without a full
/// `php_request_startup()`, pairing with [`worker_request_shutdown`].
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
