use crate::ffi::{
    ext_php_rs_zend_bailout, ext_php_rs_zend_first_try_catch, ext_php_rs_zend_try_catch,
};
use std::ffi::c_void;
use std::panic::{UnwindSafe, catch_unwind, resume_unwind};
use std::ptr::null_mut;

/// Error returned when a bailout occurs
#[derive(Debug)]
pub struct CatchError;

pub(crate) unsafe extern "C" fn panic_wrapper<R, F: FnOnce() -> R + UnwindSafe>(
    ctx: *const c_void,
) -> *const c_void {
    // we try to catch panic here so we correctly shutdown php if it happens
    // mandatory when we do assert on test as other test would not run correctly
    // SAFETY: We read the closure from the pointer and consume it. This is safe because
    // the closure is only called once.
    let func = unsafe { std::ptr::read(ctx.cast::<F>()) };
    let panic = catch_unwind(func);

    Box::into_raw(Box::new(panic)).cast::<c_void>()
}

/// PHP proposes a try catch mechanism in C using setjmp and longjmp (bailout)
/// It stores the arg of setjmp into the bailout field of the global executor
/// If a bailout is triggered, the executor will jump to the setjmp and restore
/// the previous setjmp
///
/// [`try_catch`] allows to use this mechanism
///
/// # Returns
///
/// * The result of the function
///
/// # Errors
///
/// * [`CatchError`] - A bailout occurred during the execution
pub fn try_catch<R, F: FnOnce() -> R + UnwindSafe>(func: F) -> Result<R, CatchError> {
    do_try_catch(func, false)
}

/// PHP proposes a try catch mechanism in C using setjmp and longjmp (bailout)
/// It stores the arg of setjmp into the bailout field of the global executor
/// If a bailout is triggered, the executor will jump to the setjmp and restore
/// the previous setjmp
///
/// [`try_catch_first`] allows to use this mechanism
///
/// This functions differs from [`try_catch`] as it also initialize the bailout
/// mechanism for the first time
///
/// # Returns
///
/// * The result of the function
///
/// # Errors
///
/// * [`CatchError`] - A bailout occurred during the execution
pub fn try_catch_first<R, F: FnOnce() -> R + UnwindSafe>(func: F) -> Result<R, CatchError> {
    do_try_catch(func, true)
}

fn do_try_catch<R, F: FnOnce() -> R + UnwindSafe>(func: F, first: bool) -> Result<R, CatchError> {
    let mut panic_ptr = null_mut();
    let has_bailout = unsafe {
        if first {
            ext_php_rs_zend_first_try_catch(
                panic_wrapper::<R, F>,
                (&raw const func).cast::<c_void>(),
                &raw mut panic_ptr,
            )
        } else {
            ext_php_rs_zend_try_catch(
                panic_wrapper::<R, F>,
                (&raw const func).cast::<c_void>(),
                &raw mut panic_ptr,
            )
        }
    };

    // Prevent the closure from being dropped here since it was consumed in panic_wrapper
    std::mem::forget(func);

    let panic = panic_ptr.cast::<std::thread::Result<R>>();

    // can be null if there is a bailout
    if panic.is_null() || has_bailout {
        return Err(CatchError);
    }

    match unsafe { *Box::from_raw(panic.cast::<std::thread::Result<R>>()) } {
        Ok(r) => Ok(r),
        Err(err) => {
            // we resume the panic here so it can be caught correctly by the test framework
            resume_unwind(err);
        }
    }
}

/// Trigger a bailout
///
/// This function will stop the execution of the current script
/// and jump to the last try catch block
///
/// # Safety
///
/// This function is unsafe because it can cause memory leaks
/// Since it will jump to the last try catch block, it will not call the
/// destructor of the current scope
///
/// When using this function you should ensure that all the memory allocated in
/// the current scope is released
pub unsafe fn bailout() -> ! {
    unsafe { ext_php_rs_zend_bailout() };
}

#[cfg(feature = "embed")]
#[cfg(test)]
mod tests {
    use crate::embed::Embed;
    use crate::zend::{bailout, try_catch};
    use std::ptr::null_mut;

    #[test]
    fn test_catch() {
        Embed::run(|| {
            let catch = try_catch(|| {
                unsafe {
                    bailout();
                }

                #[allow(unreachable_code)]
                #[allow(clippy::assertions_on_constants)]
                {
                    assert!(false);
                }
            });

            assert!(catch.is_err());
        });
    }

    #[test]
    fn test_no_catch() {
        Embed::run(|| {
            let catch = try_catch(|| {
                #[allow(clippy::assertions_on_constants)]
                {
                    assert!(true);
                }
            });

            assert!(catch.is_ok());
        });
    }

    #[test]
    fn test_bailout() {
        Embed::run(|| {
            unsafe {
                bailout();
            }

            #[allow(unreachable_code)]
            #[allow(clippy::assertions_on_constants)]
            {
                assert!(false);
            }
        });
    }

    #[test]
    #[should_panic(expected = "should panic")]
    fn test_panic() {
        Embed::run(|| {
            let _ = try_catch(|| {
                panic!("should panic");
            });
        });
    }

    #[test]
    fn test_return() {
        let foo = Embed::run(|| {
            let result = try_catch(|| "foo");

            assert!(result.is_ok());

            #[allow(clippy::unwrap_used)]
            result.unwrap()
        });

        assert_eq!(foo, "foo");
    }

    #[test]
    fn test_memory_leak() {
        use std::panic::AssertUnwindSafe;

        Embed::run(|| {
            let mut ptr = null_mut();

            let _ = try_catch(AssertUnwindSafe(|| {
                let mut result = "foo".to_string();
                ptr = &raw mut result;

                unsafe {
                    bailout();
                }
            }));

            // Check that the string is never released
            let result = unsafe { &*ptr as &str };

            assert_eq!(result, "foo");
        });
    }
}
