//! A hook that runs at the start of every request, through
//! `sapi_module.activate`.

use std::{
    ffi::c_int,
    fmt, mem,
    panic::AssertUnwindSafe,
    ptr,
    sync::{
        OnceLock,
        atomic::{AtomicPtr, Ordering},
    },
};

use crate::{
    error::{Error, Result},
    ffi::{
        MODULE_TEMPORARY, ZEND_RESULT_CODE_SUCCESS, ext_php_rs_sapi_globals,
        zend_unregister_ini_entries_ex,
    },
    zend::{CatchError, SapiModule, SapiRequestInfo, bailout, globals::lock, try_catch},
};

type Activate = unsafe extern "C" fn() -> c_int;
type Shutdown = unsafe extern "C" fn(c_int, c_int) -> c_int;

/// The closure given to
/// [`ModuleBuilder::sapi_activate_function`](crate::builders::ModuleBuilder::sapi_activate_function).
pub(crate) struct ActivateCallback(Box<dyn Fn(&SapiRequestInfo) + Send + Sync>);

impl ActivateCallback {
    pub(crate) fn new<F>(callback: F) -> Self
    where
        F: Fn(&SapiRequestInfo) + Send + Sync + 'static,
    {
        Self(Box::new(callback))
    }
}

impl fmt::Debug for ActivateCallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ActivateCallback")
    }
}

static CALLBACK: OnceLock<ActivateCallback> = OnceLock::new();
static USER_SHUTDOWN: OnceLock<Shutdown> = OnceLock::new();
static PREVIOUS: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

/// Keeps `callback` for the hook, and returns the MSHUTDOWN of the module:
/// the one of the user when there is no hook, else a wrapper that also removes
/// the hook.
pub(crate) fn shutdown_func(
    callback: Option<ActivateCallback>,
    user_shutdown: Option<Shutdown>,
) -> Option<Shutdown> {
    let Some(callback) = callback else {
        return user_shutdown;
    };
    let _ = CALLBACK.set(callback);
    if let Some(shutdown) = user_shutdown {
        let _ = USER_SHUTDOWN.set(shutdown);
    }
    Some(module_shutdown)
}

/// Puts the hook in `sapi_module.activate` and keeps the handler it replaces.
///
/// # Safety
///
/// Call it only from MINIT, before any request runs on another thread.
pub(crate) unsafe fn install(ty: c_int) -> Result<()> {
    if CALLBACK.get().is_none() {
        return Ok(());
    }
    if ty == MODULE_TEMPORARY.cast_signed() {
        return Err(Error::SapiActivateUnderDl);
    }

    let ours: Activate = activate;
    let mut module = SapiModule::get_mut();
    let previous = module
        .activate
        .filter(|&previous| !ptr::fn_addr_eq(previous, ours))
        .map_or(ptr::null_mut(), |previous| previous as *mut ());
    PREVIOUS.store(previous, Ordering::Release);
    module.activate = Some(ours);
    Ok(())
}

/// Puts back the handler that [`install`] replaced, if the hook is still the
/// current one. When another extension put its own hook over this one, both
/// stay, so that its chain still reaches the SAPI handler.
///
/// # Safety
///
/// Call it only from MSHUTDOWN, after the last request.
unsafe fn uninstall() {
    let ours: Activate = activate;
    let mut module = SapiModule::get_mut();
    if module
        .activate
        .is_some_and(|current| ptr::fn_addr_eq(current, ours))
    {
        module.activate = previous();
        PREVIOUS.store(ptr::null_mut(), Ordering::Release);
    }
}

fn previous() -> Option<Activate> {
    let previous = PREVIOUS.load(Ordering::Acquire);
    // SAFETY: `PREVIOUS` only holds null or an `Activate` stored by `install`,
    // and function pointers are pointer-sized.
    (!previous.is_null()).then(|| unsafe { mem::transmute::<*mut (), Activate>(previous) })
}

fn sapi_globals_borrowed_mut() -> bool {
    cfg_if::cfg_if! {
        if #[cfg(php_zts)] {
            lock::SAPI_GLOBALS_LOCK.with(|lock| lock.is_locked_exclusive())
        } else {
            lock::SAPI_GLOBALS_LOCK.is_locked_exclusive()
        }
    }
}

unsafe extern "C" fn activate() -> c_int {
    // SAFETY: `sapi_activate` would call the previous handler the same way.
    let result = previous().map_or(ZEND_RESULT_CODE_SUCCESS, |previous| unsafe { previous() });
    let Some(callback) = CALLBACK.get() else {
        return result;
    };
    if sapi_globals_borrowed_mut() {
        return result;
    }

    // SAFETY: the SAPI globals of this thread are live during `sapi_activate`,
    // and no Rust `&mut` to them is alive. The copy only holds pointers to
    // strings that the SAPI keeps until the request ends.
    let info = unsafe { (*ext_php_rs_sapi_globals()).request_info };
    if let Err(CatchError::Bailout) = try_catch(AssertUnwindSafe(|| (callback.0)(&info))) {
        // SAFETY: this frame holds only `Copy` values, so the jump skips no
        // destructor. The request startup of PHP and FrankenPHP calls
        // `sapi_activate` inside a `zend_try`. Without one, the engine ends
        // the process, as for any other bailout at this point.
        unsafe { bailout() }
    }
    result
}

/// MSHUTDOWN of a module that has the hook. It removes the hook, then runs the
/// shutdown function of the user.
unsafe extern "C" fn module_shutdown(ty: c_int, module_number: c_int) -> c_int {
    // SAFETY: the engine runs MSHUTDOWN after the last request.
    unsafe { uninstall() };
    if let Some(shutdown) = USER_SHUTDOWN.get() {
        // SAFETY: the engine would call the user function with these arguments.
        return unsafe { shutdown(ty, module_number) };
    }
    if ty == MODULE_TEMPORARY.cast_signed() {
        // SAFETY: the engine does this for a temporary module that has no
        // shutdown function.
        unsafe { zend_unregister_ini_entries_ex(module_number, ty) };
    }
    ZEND_RESULT_CODE_SUCCESS
}

#[cfg(all(test, feature = "embed"))]
mod tests {
    use super::*;
    use crate::{embed::Embed, ffi::MODULE_PERSISTENT};

    #[test]
    fn second_install_does_not_chain_to_itself() {
        let chained_to_itself = Embed::run(|| {
            let _ = shutdown_func(Some(ActivateCallback::new(|_| {})), None);
            // SAFETY: `Embed::run` holds the lock that every embed test takes,
            // so no other request starts until `uninstall`.
            unsafe {
                install(MODULE_PERSISTENT.cast_signed()).expect("first install failed");
                install(MODULE_PERSISTENT.cast_signed()).expect("second install failed");
            }
            let ours: Activate = activate;
            let chained = previous().is_some_and(|previous| ptr::fn_addr_eq(previous, ours));
            // SAFETY: see above.
            unsafe { uninstall() };
            chained
        });

        assert!(!chained_to_itself);
    }

    #[test]
    fn install_refuses_dl_module() {
        let refused = Embed::run(|| {
            let _ = shutdown_func(Some(ActivateCallback::new(|_| {})), None);
            // SAFETY: the refusal returns before it writes the SAPI module.
            let result = unsafe { install(MODULE_TEMPORARY.cast_signed()) };
            matches!(result, Err(Error::SapiActivateUnderDl))
        });

        assert!(refused);
    }
}
