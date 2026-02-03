//! Types related to callables in PHP (anonymous functions, functions, etc).

use std::{convert::TryFrom, ops::Deref, ptr};

use crate::{
    convert::{FromZval, IntoZvalDyn},
    error::{Error, Result},
    ffi::_call_user_function_impl,
    flags::DataType,
    zend::ExecutorGlobals,
};

use super::{ZendHashTable, Zval};

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

    /// Attempts to call the callable with both positional and named arguments.
    ///
    /// This method supports PHP 8.0+ named arguments, allowing you to pass
    /// arguments by name rather than position. Named arguments are passed
    /// after positional arguments.
    ///
    /// # Parameters
    ///
    /// * `params` - A list of positional parameters to call the function with.
    /// * `named_params` - A list of named parameters as (name, value) tuples.
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
    /// * If a parameter name contains a NUL byte.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendCallable;
    ///
    /// // Call str_replace with named arguments
    /// let str_replace = ZendCallable::try_from_name("str_replace").unwrap();
    /// let result = str_replace.try_call_with_named(
    ///     &[],  // no positional args
    ///     &[("search", &"world"), ("replace", &"PHP"), ("subject", &"Hello world")],
    /// ).unwrap();
    /// assert_eq!(result.string(), Some("Hello PHP".into()));
    /// ```
    // TODO: Measure this
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn try_call_with_named(
        &self,
        params: &[&dyn IntoZvalDyn],
        named_params: &[(&str, &dyn IntoZvalDyn)],
    ) -> Result<Zval> {
        if !self.0.is_callable() {
            return Err(Error::Callable);
        }

        let mut retval = Zval::new();
        let len = params.len();
        let params = params
            .iter()
            .map(|val| val.as_zval(false))
            .collect::<Result<Vec<_>>>()?;
        let packed = params.into_boxed_slice();

        // Build the named parameters hash table
        let named_ht = if named_params.is_empty() {
            None
        } else {
            let mut ht = ZendHashTable::new();
            for &(name, val) in named_params {
                let zval = val.as_zval(false)?;
                ht.insert(name, zval)?;
            }
            Some(ht)
        };

        let named_ptr = named_ht
            .as_ref()
            .map_or(ptr::null_mut(), |ht| ptr::from_ref(&**ht).cast_mut());

        let result = unsafe {
            #[allow(clippy::used_underscore_items)]
            _call_user_function_impl(
                ptr::null_mut(),
                ptr::from_ref(self.0.as_ref()).cast_mut(),
                &raw mut retval,
                len.try_into()?,
                packed.as_ptr().cast_mut(),
                named_ptr,
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

    /// Attempts to call the callable with only named arguments.
    ///
    /// This is a convenience method equivalent to calling
    /// [`try_call_with_named`] with an empty positional arguments vector.
    ///
    /// # Parameters
    ///
    /// * `named_params` - A list of named parameters as (name, value) tuples.
    ///
    /// # Returns
    ///
    /// Returns the result wrapped in [`Ok`] upon success.
    ///
    /// # Errors
    ///
    /// * If calling the callable fails, or an exception is thrown, an [`Err`]
    ///   is returned.
    /// * If a parameter name contains a NUL byte.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendCallable;
    ///
    /// // Call array_fill with named arguments only
    /// let array_fill = ZendCallable::try_from_name("array_fill").unwrap();
    /// let result = array_fill.try_call_named(&[
    ///     ("start_index", &0i64),
    ///     ("count", &3i64),
    ///     ("value", &"PHP"),
    /// ]).unwrap();
    /// ```
    ///
    /// [`try_call_with_named`]: #method.try_call_with_named
    #[inline]
    pub fn try_call_named(&self, named_params: &[(&str, &dyn IntoZvalDyn)]) -> Result<Zval> {
        self.try_call_with_named(&[], named_params)
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
