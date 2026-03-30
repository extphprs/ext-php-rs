//! Separated zval wrapper for COW-safe mutation without PHP pass-by-reference.

use std::ops::{Deref, DerefMut};

use crate::convert::FromZvalMut;
use crate::flags::DataType;
use crate::types::Zval;

/// A zval whose underlying value can be COW-separated for safe mutation.
///
/// Use this type in [`#[php_function]`](crate::php_function) signatures when
/// you need mutable access to a PHP value **without** requiring the caller to
/// pass by reference (`&$x`).
///
/// Unlike `&mut Zval`, which sets PHP's `ZEND_SEND_BY_REF` flag and forces the
/// caller to write `foo(&$var)`, `Separated` receives the value normally and
/// exposes `&mut Zval` for local mutation. Call [`Zval::array_mut`] on the
/// inner value to trigger PHP's Copy-on-Write separation before modifying
/// arrays.
///
/// # Examples
///
/// ```rust,ignore
/// use ext_php_rs::prelude::*;
/// use ext_php_rs::types::Separated;
///
/// #[php_function]
/// pub fn append_value(mut data: Separated) -> bool {
///     let Some(ht) = data.array_mut() else {
///         return false;
///     };
///     ht.push("appended").is_ok()
/// }
/// ```
///
/// PHP callers can pass literals directly:
///
/// ```php
/// append_value([1, 2, 3]); // works — no & required
/// ```
#[repr(transparent)]
pub struct Separated<'a>(&'a mut Zval);

impl Deref for Separated<'_> {
    type Target = Zval;

    #[inline]
    fn deref(&self) -> &Zval {
        self.0
    }
}

impl DerefMut for Separated<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Zval {
        self.0
    }
}

impl<'a> FromZvalMut<'a> for Separated<'a> {
    const TYPE: DataType = DataType::Mixed;

    #[inline]
    fn from_zval_mut(zval: &'a mut Zval) -> Option<Self> {
        Some(Separated(zval))
    }
}

impl std::fmt::Debug for Separated<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Separated")
            .field(&self.0.get_type())
            .finish()
    }
}
