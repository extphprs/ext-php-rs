//! Builder for creating functions and methods in PHP.

use std::{fmt::Debug, os::raw::c_char, ptr};

use crate::{
    convert::IntoZvalDyn,
    error::Result,
    ffi::{
        zend_call_known_function, zend_fetch_function_str, zend_function, zend_function_entry,
        zend_hash_str_find_ptr_lc,
    },
    flags::FunctionType,
    types::Zval,
};

use super::ClassEntry;

/// A Zend function entry.
pub type FunctionEntry = zend_function_entry;

impl Debug for FunctionEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("_zend_function_entry")
            .field("fname", &self.fname)
            .field("arg_info", &self.arg_info)
            .field("num_args", &self.num_args)
            .field("flags", &self.flags)
            .finish()
    }
}

impl FunctionEntry {
    /// Returns an empty function entry, signifing the end of a function list.
    #[must_use]
    pub fn end() -> Self {
        Self {
            fname: ptr::null::<c_char>(),
            handler: None,
            arg_info: ptr::null(),
            num_args: 0,
            flags: 0,
            #[cfg(php84)]
            doc_comment: ptr::null(),
            #[cfg(php84)]
            frameless_function_infos: ptr::null(),
        }
    }

    /// Converts the function entry into a raw and pointer, releasing it to the
    /// C world.
    #[must_use]
    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }
}

/// PHP function.
pub type Function = zend_function;

impl Function {
    /// Returns the function type.
    #[must_use]
    pub fn function_type(&self) -> FunctionType {
        FunctionType::from(unsafe { self.type_ })
    }

    /// Attempts to fetch a [`Function`] from the function name.
    #[inline]
    #[must_use]
    pub fn try_from_function(name: &str) -> Option<Self> {
        unsafe {
            let res = zend_fetch_function_str(name.as_ptr().cast::<c_char>(), name.len());
            if res.is_null() {
                return None;
            }
            Some(*res)
        }
    }

    /// Attempts to fetch a [`Function`] from the class and method name.
    #[must_use]
    pub fn try_from_method(class: &str, name: &str) -> Option<Self> {
        match ClassEntry::try_find(class) {
            None => None,
            Some(ce) => unsafe {
                let res = zend_hash_str_find_ptr_lc(
                    &raw const ce.function_table,
                    name.as_ptr().cast::<c_char>(),
                    name.len(),
                )
                .cast::<zend_function>();
                if res.is_null() {
                    return None;
                }
                Some(*res)
            },
        }
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
    /// * If the function call fails, an [`Err`] is returned.
    /// * If the number of parameters is not a valid `u32` value.
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
        let mut retval = Zval::new();
        let len = params.len();
        let params = params
            .into_iter()
            .map(|val| val.as_zval(false))
            .collect::<Result<Vec<_>>>()?;
        let packed = params.into_boxed_slice();

        unsafe {
            zend_call_known_function(
                ptr::from_ref(self).cast_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                &raw mut retval,
                len.try_into()?,
                packed.as_ptr().cast_mut(),
                ptr::null_mut(),
            );
        };

        Ok(retval)
    }
}
