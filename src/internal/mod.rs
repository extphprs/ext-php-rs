//! Internal, public functions that are called from downstream extensions.
use parking_lot::{Mutex, const_mutex};

use crate::builders::ModuleStartup;

pub mod class;
pub mod function;
pub mod property;

/// A mutex type that contains a [`ModuleStartup`] instance.
pub type ModuleStartupMutex = Mutex<Option<ModuleStartup>>;

/// The initialisation value for [`ModuleStartupMutex`]. By default the mutex
/// contains [`None`].
#[allow(clippy::declare_interior_mutable_const)]
pub const MODULE_STARTUP_INIT: ModuleStartupMutex = const_mutex(None);

/// Called by startup functions registered with the [`#[php_startup]`] macro.
/// Initializes all classes that are defined by ext-php-rs (i.e. `Closure`).
///
/// [`#[php_startup]`]: `crate::php_startup`
// TODO: Measure this
#[allow(clippy::inline_always)]
#[inline(always)]
pub fn ext_php_rs_startup() {
    #[cfg(feature = "closure")]
    crate::closure::Closure::build();
}

/// Runs the registration part of the generated MINIT and maps every Rust
/// failure to the engine's `FAILURE`.
///
/// A registration error or a panic is logged as an `E_CORE_WARNING` naming the
/// cause before `FAILURE` is returned; the engine then reports
/// `Unable to start <module> module` and stops startup (or fails the `dl()`
/// request) instead of the process aborting inside an `extern "C"` frame.
#[must_use]
pub fn startup_guard(func: impl FnOnce() -> crate::error::Result<()>) -> i32 {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(func));
    let failure = match outcome {
        Ok(Ok(())) => return crate::ffi::ZEND_RESULT_CODE_SUCCESS,
        Ok(Err(err)) => format!("module startup failed: {err}"),
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string panic payload".to_owned());
            format!("module startup panicked: {message}")
        }
    };
    crate::error::php_error(&crate::flags::ErrorType::CoreWarning, &failure);
    crate::ffi::ZEND_RESULT_CODE_FAILURE
}

#[cfg(all(test, feature = "embed"))]
mod tests {
    use super::startup_guard;
    use crate::embed::Embed;
    use crate::error::Error;
    use crate::ffi::{ZEND_RESULT_CODE_FAILURE, ZEND_RESULT_CODE_SUCCESS};

    #[test]
    fn startup_guard_maps_success_to_zero() {
        let code = Embed::run(|| startup_guard(|| Ok(())));
        assert_eq!(code, ZEND_RESULT_CODE_SUCCESS);
    }

    #[test]
    fn startup_guard_maps_an_error_to_failure() {
        let code = Embed::run(|| startup_guard(|| Err(Error::InvalidScope)));
        assert_eq!(code, ZEND_RESULT_CODE_FAILURE);
    }

    #[test]
    fn startup_guard_maps_a_panic_to_failure() {
        let code = Embed::run(|| startup_guard(|| panic!("registration exploded")));
        assert_eq!(code, ZEND_RESULT_CODE_FAILURE);
    }
}

/// Startup function of the placeholder module entry that `#[php_module]`
/// returns when the real module could not be built. Logs the build error and
/// fails MINIT.
#[must_use]
pub fn failed_module_startup(reason: &str) -> i32 {
    crate::error::php_error(
        &crate::flags::ErrorType::CoreWarning,
        &format!("module could not be built: {reason}"),
    );
    crate::ffi::ZEND_RESULT_CODE_FAILURE
}
