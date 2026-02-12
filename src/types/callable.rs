//! Types related to callables in PHP (anonymous functions, functions, etc).

use std::{cell::UnsafeCell, convert::TryFrom, mem::MaybeUninit, ops::Deref, ptr};

use crate::{
    convert::{FromZval, IntoZvalDyn},
    error::{Error, Result},
    ffi::{_zend_fcall_info_cache, zend_call_function, zend_fcall_info, zend_is_callable_ex},
    flags::DataType,
    zend::ExecutorGlobals,
};

use super::Zval;

/// Cached function call information for efficient repeated calls.
/// Contains the pre-resolved function info that can be reused across calls.
struct FunctionCache {
    fci: zend_fcall_info,
    fcc: _zend_fcall_info_cache,
}

/// Acts as a wrapper around a callable [`Zval`]. Allows the owner to call the
/// [`Zval`] as if it was a PHP function through the [`try_call`] method.
///
/// The callable lazily caches function resolution on first call for efficient
/// repeated invocations.
///
/// [`try_call`]: #method.try_call
pub struct ZendCallable<'a> {
    callable: OwnedZval<'a>,
    cache: UnsafeCell<Option<FunctionCache>>,
}

impl std::fmt::Debug for ZendCallable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZendCallable")
            .field("callable", &self.callable)
            .finish_non_exhaustive()
    }
}

impl<'a> ZendCallable<'a> {
    /// Attempts to create a new [`ZendCallable`] from a zval.
    ///
    /// # Parameters
    ///
    /// * `callable` - The underlying [`Zval`] that is callable.
    ///
    /// # Errors
    ///
    /// Returns an error if the [`Zval`] was not callable.
    pub fn new(callable: &'a Zval) -> Result<Self> {
        if callable.is_callable() {
            Ok(Self {
                callable: OwnedZval::Reference(callable),
                cache: UnsafeCell::new(None),
            })
        } else {
            Err(Error::Callable)
        }
    }

    /// Attempts to create a new [`ZendCallable`] by taking ownership of a Zval.
    /// Returns a result containing the callable if the zval was callable.
    ///
    /// # Parameters
    ///
    /// * `callable` - The underlying [`Zval`] that is callable.
    ///
    /// # Errors
    ///
    /// * [`Error::Callable`] - If the zval was not callable.
    pub fn new_owned(callable: Zval) -> Result<Self> {
        if callable.is_callable() {
            Ok(Self {
                callable: OwnedZval::Owned(callable),
                cache: UnsafeCell::new(None),
            })
        } else {
            Err(Error::Callable)
        }
    }

    /// Attempts to create a new [`ZendCallable`] from a function name. Returns
    /// a result containing the callable if the function existed and was
    /// callable.
    ///
    /// # Parameters
    ///
    /// * `name` - Name of the callable function.
    ///
    /// # Errors
    ///
    /// Returns an error if the function does not exist or is not callable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendCallable;
    ///
    /// let strpos = ZendCallable::try_from_name("strpos").unwrap();
    /// let result = strpos.try_call(vec![&"hello", &"e"]).unwrap();
    /// assert_eq!(result.long(), Some(1));
    /// ```
    pub fn try_from_name(name: &str) -> Result<Self> {
        let mut callable = Zval::new();
        callable.set_string(name, false)?;

        Self::new_owned(callable)
    }

    /// Attempts to call the callable with a list of arguments to pass to the
    /// function.
    ///
    /// You should not call this function directly, rather through the
    /// [`call_user_func`] macro.
    ///
    /// # Parameters
    ///
    /// * `params` - A list of parameters to call the function with.
    ///
    /// # Returns
    ///
    /// Returns the result wrapped in [`Ok`] upon success.
    ///
    /// # Errors
    ///
    /// * If calling the callable fails, or an exception is thrown, an [`Err`]
    ///   is returned.
    /// * If the number of parameters exceeds `u32::MAX`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendCallable;
    ///
    /// let strpos = ZendCallable::try_from_name("strpos").unwrap();
    /// let result = strpos.try_call(vec![&"hello", &"e"]).unwrap();
    /// assert_eq!(result.long(), Some(1));
    /// ```
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    pub fn try_call(&self, params: Vec<&dyn IntoZvalDyn>) -> Result<Zval> {
        // SAFETY: We have exclusive access through &self and PHP extensions are
        // single-threaded per request. The UnsafeCell allows interior mutability
        // for the lazy cache initialization.
        let cache = unsafe { &mut *self.cache.get() };

        // Initialize cache on first call if needed
        if cache.is_none() {
            *cache = Some(self.init_cache()?);
        }

        // SAFETY: We just ensured cache is Some above
        let Some(cached) = cache else {
            unreachable!("cache was just initialized");
        };

        let mut retval = Zval::new();

        // Convert parameters to zvals
        let params: Vec<Zval> = params
            .into_iter()
            .map(|val| val.as_zval(false))
            .collect::<Result<Vec<_>>>()?;

        // Update with call-specific data
        cached.fci.retval = &raw mut retval;
        cached.fci.params = params.as_ptr().cast_mut();
        cached.fci.param_count = params.len() as u32;

        // Call the function using the cached info
        let result = unsafe { zend_call_function(&raw mut cached.fci, &raw mut cached.fcc) };

        // Reset fci pointers to avoid dangling references
        cached.fci.retval = ptr::null_mut();
        cached.fci.params = ptr::null_mut();
        cached.fci.param_count = 0;

        if result != 0 {
            Err(Error::Callable)
        } else if let Some(e) = ExecutorGlobals::take_exception() {
            Err(Error::Exception(e))
        } else {
            Ok(retval)
        }
    }

    /// Initializes the function cache by resolving the callable.
    fn init_cache(&self) -> Result<FunctionCache> {
        let callable = self.callable.as_ref();
        let mut fcc = MaybeUninit::<_zend_fcall_info_cache>::uninit();

        // Check if callable and initialize the cache
        let is_callable = unsafe {
            zend_is_callable_ex(
                ptr::from_ref(callable).cast_mut(),
                ptr::null_mut(), // object
                0,               // check_flags
                ptr::null_mut(), // callable_name (we don't need it)
                fcc.as_mut_ptr(),
                ptr::null_mut(), // error (we don't need detailed error)
            )
        };

        if !is_callable {
            return Err(Error::Callable);
        }

        // SAFETY: fcc was initialized by zend_is_callable_ex when it returned true
        let fcc = unsafe { fcc.assume_init() };

        // Initialize fci with the callable
        let fci = zend_fcall_info {
            size: std::mem::size_of::<zend_fcall_info>(),
            function_name: callable.shallow_clone(),
            retval: ptr::null_mut(),
            params: ptr::null_mut(),
            object: fcc.object,
            param_count: 0,
            named_params: ptr::null_mut(),
        };

        Ok(FunctionCache { fci, fcc })
    }
}

impl<'a> FromZval<'a> for ZendCallable<'a> {
    const TYPE: DataType = DataType::Callable;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        ZendCallable::new(zval).ok()
    }
}

impl TryFrom<Zval> for ZendCallable<'_> {
    type Error = Error;

    fn try_from(value: Zval) -> Result<Self> {
        ZendCallable::new_owned(value)
    }
}

/// A container for a zval. Either contains a reference to a zval or an owned
/// zval.
#[derive(Debug)]
enum OwnedZval<'a> {
    Reference(&'a Zval),
    Owned(Zval),
}

impl OwnedZval<'_> {
    fn as_ref(&self) -> &Zval {
        match self {
            OwnedZval::Reference(zv) => zv,
            OwnedZval::Owned(zv) => zv,
        }
    }
}

impl Deref for OwnedZval<'_> {
    type Target = Zval;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

/// A cached callable that eagerly pre-computes function resolution for
/// efficient repeated calls.
///
/// This type resolves the function at construction time, allowing early
/// validation that the callable exists. Use this when you want to fail fast
/// if a function doesn't exist, rather than discovering the error on first
/// call.
///
/// # Comparison with [`ZendCallable`]
///
/// Both types cache function resolution for efficient repeated calls:
/// - `ZendCallable`: Lazy caching - resolves on first call, validates at call time
/// - `CachedCallable`: Eager caching - resolves at construction, validates upfront
///
/// # Example
///
/// ```no_run
/// use ext_php_rs::types::CachedCallable;
///
/// let mut callback = CachedCallable::try_from_name("strtoupper").unwrap();
///
/// // Each call reuses the cached function info
/// for word in ["hello", "world", "rust"] {
///     let result = callback.try_call(vec![&word]).unwrap();
///     println!("Upper: {}", result.string().unwrap());
/// }
/// ```
pub struct CachedCallable {
    /// The callable zval (keeps the callable alive)
    callable: Zval,
    /// Cached function call info cache (only the fcc is stored, fci is created on the stack
    /// for each call to avoid UB from re-entrancy)
    fcc: _zend_fcall_info_cache,
}

impl std::fmt::Debug for CachedCallable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedCallable")
            .field("callable", &self.callable)
            .finish_non_exhaustive()
    }
}

impl CachedCallable {
    /// Creates a new cached callable from a function name.
    ///
    /// This resolves the function once and caches the result for efficient
    /// repeated calls.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the function to call.
    ///
    /// # Errors
    ///
    /// Returns an error if the function does not exist or is not callable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::CachedCallable;
    ///
    /// let mut strpos = CachedCallable::try_from_name("strpos").unwrap();
    /// let result = strpos.try_call(vec![&"hello world", &"world"]).unwrap();
    /// assert_eq!(result.long(), Some(6));
    /// ```
    pub fn try_from_name(name: &str) -> Result<Self> {
        let mut callable = Zval::new();
        callable.set_string(name, false)?;
        Self::try_from_zval(callable)
    }

    /// Creates a new cached callable from a zval.
    ///
    /// The zval can be a string (function name), array (class method), or
    /// closure.
    ///
    /// # Parameters
    ///
    /// * `callable` - The zval representing the callable.
    ///
    /// # Errors
    ///
    /// Returns an error if the zval is not callable.
    pub fn try_from_zval(callable: Zval) -> Result<Self> {
        let mut fcc = MaybeUninit::<_zend_fcall_info_cache>::uninit();

        // Check if callable and initialize the cache
        let is_callable = unsafe {
            zend_is_callable_ex(
                ptr::from_ref(&callable).cast_mut(),
                ptr::null_mut(), // object
                0,               // check_flags
                ptr::null_mut(), // callable_name (we don't need it)
                fcc.as_mut_ptr(),
                ptr::null_mut(), // error (we don't need detailed error)
            )
        };

        if !is_callable {
            return Err(Error::Callable);
        }

        // SAFETY: fcc was initialized by zend_is_callable_ex when it returned true
        let fcc = unsafe { fcc.assume_init() };

        Ok(Self { callable, fcc })
    }

    /// Calls the cached callable with the given parameters.
    ///
    /// This method is optimized for repeated calls - the function lookup is
    /// cached and only parameter binding and invocation occur on each call.
    ///
    /// # Parameters
    ///
    /// * `params` - A list of parameters to pass to the function.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The call fails
    /// * An exception is thrown during execution
    /// * Parameter conversion fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::CachedCallable;
    ///
    /// let mut callback = CachedCallable::try_from_name("strtoupper").unwrap();
    ///
    /// // Efficient repeated calls
    /// let words = vec!["hello", "world", "rust"];
    /// for word in words {
    ///     let result = callback.try_call(vec![&word]).unwrap();
    ///     println!("{}", result.string().unwrap());
    /// }
    /// ```
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    pub fn try_call(&mut self, params: Vec<&dyn IntoZvalDyn>) -> Result<Zval> {
        let mut retval = Zval::new();

        // Convert parameters to zvals
        let params: Vec<Zval> = params
            .into_iter()
            .map(|val| val.as_zval(false))
            .collect::<Result<Vec<_>>>()?;

        // Create fci on the stack for each call to avoid UB from re-entrancy.
        // If a callback calls back into this same callable, each call needs its own fci.
        let mut fci = zend_fcall_info {
            size: std::mem::size_of::<zend_fcall_info>(),
            function_name: self.callable.shallow_clone(),
            retval: &raw mut retval,
            params: params.as_ptr().cast_mut(),
            object: self.fcc.object,
            param_count: params.len() as u32,
            named_params: ptr::null_mut(),
        };

        // Call the function using the stack-local fci and cached fcc
        let result = unsafe { zend_call_function(&raw mut fci, &raw mut self.fcc) };

        if result != 0 {
            Err(Error::Callable)
        } else if let Some(e) = ExecutorGlobals::take_exception() {
            Err(Error::Exception(e))
        } else {
            Ok(retval)
        }
    }

    /// Calls the cached callable with pre-converted zval parameters.
    ///
    /// This is the most efficient way to call a function when you already have
    /// zval arguments, as it avoids any parameter conversion overhead.
    ///
    /// # Parameters
    ///
    /// * `params` - A slice of Zval parameters to pass to the function.
    ///
    /// # Errors
    ///
    /// Returns an error if the call fails or an exception is thrown.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::{CachedCallable, Zval};
    ///
    /// let mut callback = CachedCallable::try_from_name("array_sum").unwrap();
    ///
    /// let mut arr = Zval::new();
    /// // ... populate arr as a PHP array ...
    ///
    /// let result = callback.call_with_zvals(&[arr]).unwrap();
    /// ```
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    pub fn call_with_zvals(&mut self, params: &[Zval]) -> Result<Zval> {
        let mut retval = Zval::new();

        // Create fci on the stack for each call to avoid UB from re-entrancy.
        // If a callback calls back into this same callable, each call needs its own fci.
        let mut fci = zend_fcall_info {
            size: std::mem::size_of::<zend_fcall_info>(),
            function_name: self.callable.shallow_clone(),
            retval: &raw mut retval,
            params: params.as_ptr().cast_mut(),
            object: self.fcc.object,
            param_count: params.len() as u32,
            named_params: ptr::null_mut(),
        };

        // Call the function using the stack-local fci and cached fcc
        let result = unsafe { zend_call_function(&raw mut fci, &raw mut self.fcc) };

        if result != 0 {
            Err(Error::Callable)
        } else if let Some(e) = ExecutorGlobals::take_exception() {
            Err(Error::Exception(e))
        } else {
            Ok(retval)
        }
    }
}

impl TryFrom<Zval> for CachedCallable {
    type Error = Error;

    fn try_from(value: Zval) -> Result<Self> {
        CachedCallable::try_from_zval(value)
    }
}

impl TryFrom<&str> for CachedCallable {
    type Error = Error;

    fn try_from(name: &str) -> Result<Self> {
        CachedCallable::try_from_name(name)
    }
}

#[cfg(test)]
#[cfg(feature = "embed")]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    use crate::embed::Embed;

    #[test]
    fn test_zend_callable_new_non_callable() {
        Embed::run(|| {
            let zval = Zval::new();
            let result = ZendCallable::new(&zval);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_zend_callable_new_owned_non_callable() {
        Embed::run(|| {
            let zval = Zval::new();
            let result = ZendCallable::new_owned(zval);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_zend_callable_try_from_name() {
        Embed::run(|| {
            let callable = ZendCallable::try_from_name("strtoupper");
            assert!(callable.is_ok());
        });
    }

    #[test]
    fn test_zend_callable_try_from_name_invalid() {
        Embed::run(|| {
            let callable = ZendCallable::try_from_name("nonexistent_function_12345");
            assert!(callable.is_err());
        });
    }

    #[test]
    fn test_zend_callable_try_call() {
        Embed::run(|| {
            let callable = ZendCallable::try_from_name("strtoupper").unwrap();
            let result = callable.try_call(vec![&"hello"]);
            assert!(result.is_ok());
            let zval = result.unwrap();
            assert_eq!(zval.string().unwrap().clone(), "HELLO");
        });
    }

    #[test]
    fn test_zend_callable_multiple_calls() {
        Embed::run(|| {
            let callable = ZendCallable::try_from_name("strlen").unwrap();

            // Call multiple times to verify lazy caching works
            for (input, expected) in [("hello", 5), ("world!", 6), ("", 0), ("rust", 4)] {
                let result = callable.try_call(vec![&input]).unwrap();
                assert_eq!(result.long().unwrap(), expected);
            }
        });
    }

    // =========================================================================
    // CachedCallable tests
    // =========================================================================

    #[test]
    fn test_cached_callable_try_from_name() {
        Embed::run(|| {
            let callable = CachedCallable::try_from_name("strtoupper");
            assert!(callable.is_ok());
        });
    }

    #[test]
    fn test_cached_callable_try_from_name_invalid() {
        Embed::run(|| {
            let callable = CachedCallable::try_from_name("nonexistent_function_12345");
            assert!(callable.is_err());
        });
    }

    #[test]
    fn test_cached_callable_try_call() {
        Embed::run(|| {
            let mut callable = CachedCallable::try_from_name("strtoupper").unwrap();
            let result = callable.try_call(vec![&"hello"]);
            assert!(result.is_ok());
            let zval = result.unwrap();
            assert_eq!(zval.string().unwrap().clone(), "HELLO");
        });
    }

    #[test]
    fn test_cached_callable_multiple_calls() {
        Embed::run(|| {
            let mut callable = CachedCallable::try_from_name("strlen").unwrap();

            // Call multiple times to verify caching works
            for (input, expected) in [("hello", 5), ("world!", 6), ("", 0), ("rust", 4)] {
                let result = callable.try_call(vec![&input]).unwrap();
                assert_eq!(result.long().unwrap(), expected);
            }
        });
    }

    #[test]
    fn test_cached_callable_call_with_zvals() {
        Embed::run(|| {
            let mut callable = CachedCallable::try_from_name("strlen").unwrap();

            let mut arg = Zval::new();
            arg.set_string("test", false).unwrap();

            let result = callable.call_with_zvals(&[arg]);
            assert!(result.is_ok());
            assert_eq!(result.unwrap().long().unwrap(), 4);
        });
    }

    #[test]
    fn test_cached_callable_try_from_zval() {
        Embed::run(|| {
            let mut zval = Zval::new();
            zval.set_string("strtolower", false).unwrap();

            let callable = CachedCallable::try_from_zval(zval);
            assert!(callable.is_ok());

            let mut callable = callable.unwrap();
            let result = callable.try_call(vec![&"HELLO"]).unwrap();
            assert_eq!(result.string().unwrap().clone(), "hello");
        });
    }

    #[test]
    fn test_cached_callable_try_from() {
        Embed::run(|| {
            let mut zval = Zval::new();
            zval.set_string("abs", false).unwrap();

            let callable: Result<CachedCallable> = zval.try_into();
            assert!(callable.is_ok());

            let mut callable = callable.unwrap();
            let result = callable.try_call(vec![&(-42i64)]).unwrap();
            assert_eq!(result.long().unwrap(), 42);
        });
    }

    #[test]
    fn test_cached_callable_debug() {
        Embed::run(|| {
            let callable = CachedCallable::try_from_name("strlen").unwrap();
            let debug_str = format!("{callable:?}");
            assert!(debug_str.contains("CachedCallable"));
        });
    }
}
