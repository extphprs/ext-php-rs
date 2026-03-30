//! PHP reference wrapper that explicitly requires pass-by-reference from callers.

use std::ops::{Deref, DerefMut};

use crate::convert::FromZvalMut;
use crate::flags::DataType;
use crate::types::Zval;

/// A PHP reference (`&$x`) that explicitly requires the caller to pass by
/// reference.
///
/// Use this type in [`#[php_function]`](crate::php_function) signatures when
/// you need to modify the caller's original variable. This is the **only** type
/// that sets PHP's `ZEND_SEND_BY_REF` flag.
///
/// Mutations through `PhpRef` affect the original variable in the caller's
/// scope. The proc macro automatically dereferences the `zend_reference`
/// wrapper before passing the inner value to your function.
///
/// # When to use `PhpRef` vs `Separated`
///
/// | Type | PHP call syntax | Caller's variable modified? |
/// |---|---|---|
/// | [`Separated`](super::Separated) | `foo($x)` or `foo([1,2])` | No |
/// | `PhpRef` | `foo(&$x)` — literals rejected | Yes |
///
/// # Examples
///
/// ```rust,ignore
/// use ext_php_rs::prelude::*;
/// use ext_php_rs::types::PhpRef;
///
/// #[php_function]
/// pub fn increment(mut val: PhpRef) {
///     if let Some(n) = val.long() {
///         val.set_long(n + 1);
///     }
/// }
/// ```
///
/// ```php
/// $x = 5;
/// increment($x); // $x is now 6
/// ```
#[repr(transparent)]
pub struct PhpRef<'a>(&'a mut Zval);

impl Deref for PhpRef<'_> {
    type Target = Zval;

    #[inline]
    fn deref(&self) -> &Zval {
        self.0
    }
}

impl DerefMut for PhpRef<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Zval {
        self.0
    }
}

impl<'a> FromZvalMut<'a> for PhpRef<'a> {
    const TYPE: DataType = DataType::Mixed;

    #[inline]
    fn from_zval_mut(zval: &'a mut Zval) -> Option<Self> {
        Some(PhpRef(zval))
    }
}

impl std::fmt::Debug for PhpRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PhpRef").field(&self.0.get_type()).finish()
    }
}
