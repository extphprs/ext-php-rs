//! Types related to callables in PHP (anonymous functions, functions, etc).

use std::{convert::TryFrom, mem::MaybeUninit, ops::Deref, ptr};

use crate::{
    convert::{FromZval, IntoZvalDyn},
    error::{Error, Result},
    ffi::{
        _call_user_function_impl, _zend_fcall_info_cache, zend_call_function, zend_fcall_info,
        zend_is_callable_ex,
    },
    flags::DataType,
    zend::ExecutorGlobals,
};

use super::Zval;

/// Acts as a wrapper around a callable [`Zval`]. Allows the owner to call the
/// [`Zval`] as if it was a PHP function through the [`try_call`] method.
///
/// [`try_call`]: #method.try_call
#[derive(Debug)]
pub struct ZendCallable<'a>(OwnedZval<'a>);

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
            Ok(Self(OwnedZval::Reference(callable)))
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
            Ok(Self(OwnedZval::Owned(callable)))
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
    // TODO: Measure this
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn try_call(&self, params: Vec<&dyn IntoZvalDyn>) -> Result<Zval> {
        if !self.0.is_callable() {
            return Err(Error::Callable);
        }

        let mut retval = Zval::new();
        let len = params.len();
        let params = params
            .into_iter()
            .map(|val| val.as_zval(false))
            .collect::<Result<Vec<_>>>()?;
        let packed = params.into_boxed_slice();

        let result = unsafe {
            #[allow(clippy::used_underscore_items)]
            _call_user_function_impl(
                ptr::null_mut(),
                ptr::from_ref(self.0.as_ref()).cast_mut(),
                &raw mut retval,
                len.try_into()?,
                packed.as_ptr().cast_mut(),
                ptr::null_mut(),
            )
        };

        if result < 0 {
            Err(Error::Callable)
        } else if let Some(e) = ExecutorGlobals::take_exception() {
            Err(Error::Exception(e))
        } else {
            Ok(retval)
        }
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

/// A cached callable that pre-computes function resolution for efficient
/// repeated calls.
///
/// Unlike [`ZendCallable`], this type caches the function lookup information in
/// `zend_fcall_info` and `zend_fcall_info_cache` structures, avoiding the
/// overhead of function resolution on each call. This is particularly
/// beneficial when calling the same function multiple times, such as in
/// iterator callbacks or event handlers.
///
/// # Performance
///
/// For a function called N times:
/// - `ZendCallable`: O(N) function lookups
/// - `CachedCallable`: O(1) function lookup + O(N) direct calls
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
    /// Cached function call info
    fci: zend_fcall_info,
    /// Cached function call info cache
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

        // Initialize fci with the callable
        // Note: We use shallow_clone to copy the callable zval into the fci structure
        let fci = zend_fcall_info {
            size: std::mem::size_of::<zend_fcall_info>(),
            function_name: callable.shallow_clone(),
            retval: ptr::null_mut(),
            params: ptr::null_mut(),
            object: fcc.object,
            param_count: 0,
            named_params: ptr::null_mut(),
        };

        Ok(Self { callable, fci, fcc })
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

        // Update fci with the current call parameters
        self.fci.retval = &raw mut retval;
        self.fci.params = params.as_ptr().cast_mut();
        self.fci.param_count = params.len() as u32;

        // Call the function using the cached info
        let result = unsafe { zend_call_function(&raw mut self.fci, &raw mut self.fcc) };

        // Reset fci pointers to avoid dangling references
        self.fci.retval = ptr::null_mut();
        self.fci.params = ptr::null_mut();
        self.fci.param_count = 0;

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

        // Update fci with the current call parameters
        self.fci.retval = &raw mut retval;
        self.fci.params = params.as_ptr().cast_mut();
        self.fci.param_count = params.len() as u32;

        // Call the function using the cached info
        let result = unsafe { zend_call_function(&raw mut self.fci, &raw mut self.fcc) };

        // Reset fci pointers
        self.fci.retval = ptr::null_mut();
        self.fci.params = ptr::null_mut();
        self.fci.param_count = 0;

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
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    // Note: ZendCallable tests that create Zval require PHP runtime.
    // These tests are marked with #[cfg(feature = "embed")] and use Embed::run().

    #[test]
    #[cfg(feature = "embed")]
    fn test_zend_callable_new_non_callable() {
        use crate::embed::Embed;

        Embed::run(|| {
            let zval = Zval::new();
            let result = ZendCallable::new(&zval);
            assert!(result.is_err());
        });
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_zend_callable_new_owned_non_callable() {
        use crate::embed::Embed;

        Embed::run(|| {
            let zval = Zval::new();
            let result = ZendCallable::new_owned(zval);
            assert!(result.is_err());
        });
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_zend_callable_try_from_name() {
        use crate::embed::Embed;

        Embed::run(|| {
            let callable = ZendCallable::try_from_name("strtoupper");
            assert!(callable.is_ok());
        });
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_zend_callable_try_from_name_invalid() {
        use crate::embed::Embed;

        Embed::run(|| {
            let callable = ZendCallable::try_from_name("nonexistent_function_12345");
            assert!(callable.is_err());
        });
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_zend_callable_try_call() {
        use crate::embed::Embed;

        Embed::run(|| {
            let callable = ZendCallable::try_from_name("strtoupper").unwrap();
            let result = callable.try_call(vec![&"hello"]);
            assert!(result.is_ok());
            let zval = result.unwrap();
            assert_eq!(zval.string().unwrap().clone(), "HELLO");
        });
    }

    // =========================================================================
    // CachedCallable tests
    // =========================================================================

    #[test]
    #[cfg(feature = "embed")]
    fn test_cached_callable_try_from_name() {
        use crate::embed::Embed;

        Embed::run(|| {
            let callable = CachedCallable::try_from_name("strtoupper");
            assert!(callable.is_ok());
        });
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_cached_callable_try_from_name_invalid() {
        use crate::embed::Embed;

        Embed::run(|| {
            let callable = CachedCallable::try_from_name("nonexistent_function_12345");
            assert!(callable.is_err());
        });
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_cached_callable_try_call() {
        use crate::embed::Embed;

        Embed::run(|| {
            let mut callable = CachedCallable::try_from_name("strtoupper").unwrap();
            let result = callable.try_call(vec![&"hello"]);
            assert!(result.is_ok());
            let zval = result.unwrap();
            assert_eq!(zval.string().unwrap().clone(), "HELLO");
        });
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_cached_callable_multiple_calls() {
        use crate::embed::Embed;

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
    #[cfg(feature = "embed")]
    fn test_cached_callable_call_with_zvals() {
        use crate::embed::Embed;

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
    #[cfg(feature = "embed")]
    fn test_cached_callable_try_from_zval() {
        use crate::embed::Embed;

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
    #[cfg(feature = "embed")]
    fn test_cached_callable_try_from() {
        use crate::embed::Embed;

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
    #[cfg(feature = "embed")]
    fn test_cached_callable_debug() {
        use crate::embed::Embed;

        Embed::run(|| {
            let callable = CachedCallable::try_from_name("strlen").unwrap();
            let debug_str = format!("{callable:?}");
            assert!(debug_str.contains("CachedCallable"));
        });
    }
}
