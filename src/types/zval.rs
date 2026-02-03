//! The base value in PHP. A Zval can contain any PHP type, and the type that it
//! contains is determined by a property inside the struct. The content of the
//! Zval is stored in a union.

use std::{convert::TryInto, ffi::c_void, fmt::Debug, ptr};

use cfg_if::cfg_if;

use crate::types::ZendIterator;
use crate::types::iterable::Iterable;
use crate::{
    binary::Pack,
    binary_slice::PackSlice,
    boxed::ZBox,
    convert::{FromZval, FromZvalMut, IntoZval, IntoZvalDyn},
    error::{Error, Result},
    ffi::{
        _zval_struct__bindgen_ty_1, _zval_struct__bindgen_ty_2, ext_php_rs_zend_string_release,
        zend_array_dup, zend_is_callable, zend_is_identical, zend_is_iterable, zend_is_true,
        zend_resource, zend_value, zval, zval_ptr_dtor,
    },
    flags::DataType,
    flags::ZvalTypeFlags,
    rc::PhpRc,
    types::{ZendCallable, ZendHashTable, ZendLong, ZendObject, ZendStr},
};

/// A zend value. This is the primary storage container used throughout the Zend
/// engine.
///
/// A zval can be thought of as a Rust enum, a type that can contain different
/// values such as integers, strings, objects etc.
pub type Zval = zval;

// TODO(david): can we make zval send+sync? main problem is that refcounted
// types do not have atomic refcounters, so technically two threads could
// reference the same object and attempt to modify refcounter at the same time.
// need to look into how ZTS works.

// unsafe impl Send for Zval {}
// unsafe impl Sync for Zval {}

impl Zval {
    /// Creates a new, empty zval.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            value: zend_value {
                ptr: ptr::null_mut(),
            },
            #[allow(clippy::used_underscore_items)]
            u1: _zval_struct__bindgen_ty_1 {
                type_info: DataType::Null.as_u32(),
            },
            #[allow(clippy::used_underscore_items)]
            u2: _zval_struct__bindgen_ty_2 { next: 0 },
        }
    }

    /// Creates a null zval
    #[must_use]
    pub fn null() -> Zval {
        let mut zval = Zval::new();
        zval.set_null();
        zval
    }

    /// Creates a zval containing an empty array.
    #[must_use]
    pub fn new_array() -> Zval {
        let mut zval = Zval::new();
        zval.set_hashtable(ZendHashTable::new());
        zval
    }

    /// Dereference the zval, if it is a reference.
    #[must_use]
    pub fn dereference(&self) -> &Self {
        self.reference().or_else(|| self.indirect()).unwrap_or(self)
    }

    /// Dereference the zval mutable, if it is a reference.
    ///
    /// # Panics
    ///
    /// Panics if a mutable reference to the zval is not possible.
    pub fn dereference_mut(&mut self) -> &mut Self {
        // TODO: probably more ZTS work is needed here
        if self.is_reference() {
            #[allow(clippy::unwrap_used)]
            return self.reference_mut().unwrap();
        }
        if self.is_indirect() {
            #[allow(clippy::unwrap_used)]
            return self.indirect_mut().unwrap();
        }
        self
    }

    /// Returns the value of the zval if it is a long.
    #[must_use]
    pub fn long(&self) -> Option<ZendLong> {
        if self.is_long() {
            Some(unsafe { self.value.lval })
        } else {
            None
        }
    }

    /// Returns the value of the zval if it is a bool.
    #[must_use]
    pub fn bool(&self) -> Option<bool> {
        if self.is_true() {
            Some(true)
        } else if self.is_false() {
            Some(false)
        } else {
            None
        }
    }

    /// Returns the value of the zval if it is a double.
    #[must_use]
    pub fn double(&self) -> Option<f64> {
        if self.is_double() {
            Some(unsafe { self.value.dval })
        } else {
            None
        }
    }

    /// Returns the value of the zval as a zend string, if it is a string.
    ///
    /// Note that this functions output will not be the same as
    /// [`string()`](#method.string), as this function does not attempt to
    /// convert other types into a [`String`].
    #[must_use]
    pub fn zend_str(&self) -> Option<&ZendStr> {
        if self.is_string() {
            unsafe { self.value.str_.as_ref() }
        } else {
            None
        }
    }

    /// Returns the value of the zval if it is a string.
    ///
    /// [`str()`]: #method.str
    pub fn string(&self) -> Option<String> {
        self.str().map(ToString::to_string)
    }

    /// Returns the value of the zval if it is a string.
    ///
    /// Note that this functions output will not be the same as
    /// [`string()`](#method.string), as this function does not attempt to
    /// convert other types into a [`String`], as it could not pass back a
    /// [`&str`] in those cases.
    #[must_use]
    pub fn str(&self) -> Option<&str> {
        self.zend_str().and_then(|zs| zs.as_str().ok())
    }

    /// Returns the value of the zval if it is a string and can be unpacked into
    /// a vector of a given type. Similar to the [`unpack`] function in PHP,
    /// except you can only unpack one type.
    ///
    /// # Safety
    ///
    /// There is no way to tell if the data stored in the string is actually of
    /// the given type. The results of this function can also differ from
    /// platform-to-platform due to the different representation of some
    /// types on different platforms. Consult the [`pack`] function
    /// documentation for more details.
    ///
    /// [`pack`]: https://www.php.net/manual/en/function.pack.php
    /// [`unpack`]: https://www.php.net/manual/en/function.unpack.php
    pub fn binary<T: Pack>(&self) -> Option<Vec<T>> {
        self.zend_str().map(T::unpack_into)
    }

    /// Returns the value of the zval if it is a string and can be unpacked into
    /// a slice of a given type. Similar to the [`unpack`] function in PHP,
    /// except you can only unpack one type.
    ///
    /// This function is similar to [`Zval::binary`] except that a slice is
    /// returned instead of a vector, meaning the contents of the string is
    /// not copied.
    ///
    /// # Safety
    ///
    /// There is no way to tell if the data stored in the string is actually of
    /// the given type. The results of this function can also differ from
    /// platform-to-platform due to the different representation of some
    /// types on different platforms. Consult the [`pack`] function
    /// documentation for more details.
    ///
    /// [`pack`]: https://www.php.net/manual/en/function.pack.php
    /// [`unpack`]: https://www.php.net/manual/en/function.unpack.php
    pub fn binary_slice<T: PackSlice>(&self) -> Option<&[T]> {
        self.zend_str().map(T::unpack_into)
    }

    /// Returns the value of the zval if it is a resource.
    #[must_use]
    pub fn resource(&self) -> Option<*mut zend_resource> {
        // TODO: Can we improve this function? I haven't done much research into
        // resources so I don't know if this is the optimal way to return this.
        if self.is_resource() {
            Some(unsafe { self.value.res })
        } else {
            None
        }
    }

    /// Returns an immutable reference to the underlying zval hashtable if the
    /// zval contains an array.
    #[must_use]
    pub fn array(&self) -> Option<&ZendHashTable> {
        if self.is_array() {
            unsafe { self.value.arr.as_ref() }
        } else {
            None
        }
    }

    /// Returns a mutable reference to the underlying zval hashtable if the zval
    /// contains an array.
    ///
    /// # Array Separation
    ///
    /// PHP arrays use copy-on-write (COW) semantics. Before returning a mutable
    /// reference, this method checks if the array is shared (refcount > 1) and
    /// if so, creates a private copy. This is equivalent to PHP's
    /// `SEPARATE_ARRAY()` macro and prevents the "Assertion failed:
    /// `zend_gc_refcount` == 1" error that occurs when modifying shared arrays.
    pub fn array_mut(&mut self) -> Option<&mut ZendHashTable> {
        if self.is_array() {
            unsafe {
                let arr = self.value.arr;
                // Check if the array is shared (refcount > 1)
                // If so, we need to separate it (copy-on-write)
                if (*arr).gc.refcount > 1 {
                    // Decrement the refcount of the original array
                    (*arr).gc.refcount -= 1;
                    // Duplicate the array to get our own private copy
                    let new_arr = zend_array_dup(arr);
                    // Update the zval to point to the new array
                    self.value.arr = new_arr;
                }
                self.value.arr.as_mut()
            }
        } else {
            None
        }
    }

    /// Returns the value of the zval if it is an object.
    #[must_use]
    pub fn object(&self) -> Option<&ZendObject> {
        if self.is_object() {
            unsafe { self.value.obj.as_ref() }
        } else {
            None
        }
    }

    /// Returns a mutable reference to the object contained in the [`Zval`], if
    /// any.
    pub fn object_mut(&mut self) -> Option<&mut ZendObject> {
        if self.is_object() {
            unsafe { self.value.obj.as_mut() }
        } else {
            None
        }
    }

    /// Attempts to call a method on the object contained in the zval.
    ///
    /// # Errors
    ///
    /// * Returns an error if the [`Zval`] is not an object.
    // TODO: Measure this
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn try_call_method(&self, name: &str, params: Vec<&dyn IntoZvalDyn>) -> Result<Zval> {
        self.object()
            .ok_or(Error::Object)?
            .try_call_method(name, params)
    }

    /// Returns the value of the zval if it is an internal indirect reference.
    #[must_use]
    pub fn indirect(&self) -> Option<&Zval> {
        if self.is_indirect() {
            Some(unsafe { &*(self.value.zv.cast::<Zval>()) })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the zval if it is an internal indirect
    /// reference.
    // TODO: Verify if this is safe to use, as it allows mutating the
    // hashtable while only having a reference to it. #461
    #[allow(clippy::mut_from_ref)]
    #[must_use]
    pub fn indirect_mut(&self) -> Option<&mut Zval> {
        if self.is_indirect() {
            Some(unsafe { &mut *(self.value.zv.cast::<Zval>()) })
        } else {
            None
        }
    }

    /// Returns the value of the zval if it is a reference.
    #[must_use]
    pub fn reference(&self) -> Option<&Zval> {
        if self.is_reference() {
            Some(&unsafe { self.value.ref_.as_ref() }?.val)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the underlying zval if it is a reference.
    pub fn reference_mut(&mut self) -> Option<&mut Zval> {
        if self.is_reference() {
            Some(&mut unsafe { self.value.ref_.as_mut() }?.val)
        } else {
            None
        }
    }

    /// Returns the value of the zval if it is callable.
    #[must_use]
    pub fn callable(&self) -> Option<ZendCallable<'_>> {
        // The Zval is checked if it is callable in the `new` function.
        ZendCallable::new(self).ok()
    }

    /// Returns an iterator over the zval if it is traversable.
    #[must_use]
    pub fn traversable(&self) -> Option<&mut ZendIterator> {
        if self.is_traversable() {
            self.object()?.get_class_entry().get_iterator(self, false)
        } else {
            None
        }
    }

    /// Returns an iterable over the zval if it is an array or traversable. (is
    /// iterable)
    #[must_use]
    pub fn iterable(&self) -> Option<Iterable<'_>> {
        if self.is_iterable() {
            Iterable::from_zval(self)
        } else {
            None
        }
    }

    /// Returns the value of the zval if it is a pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer contained in the zval is in fact
    /// a pointer to an instance of `T`, as the zval has no way of defining
    /// the type of pointer.
    #[must_use]
    pub unsafe fn ptr<T>(&self) -> Option<*mut T> {
        if self.is_ptr() {
            Some(unsafe { self.value.ptr.cast::<T>() })
        } else {
            None
        }
    }

    /// Attempts to call the zval as a callable with a list of arguments to pass
    /// to the function. Note that a thrown exception inside the callable is
    /// not detectable, therefore you should check if the return value is
    /// valid rather than unwrapping. Returns a result containing the return
    /// value of the function, or an error.
    ///
    /// You should not call this function directly, rather through the
    /// [`call_user_func`] macro.
    ///
    /// # Parameters
    ///
    /// * `params` - A list of parameters to call the function with.
    ///
    /// # Errors
    ///
    /// * Returns an error if the [`Zval`] is not callable.
    // TODO: Measure this
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn try_call(&self, params: Vec<&dyn IntoZvalDyn>) -> Result<Zval> {
        self.callable().ok_or(Error::Callable)?.try_call(params)
    }

    /// Returns the type of the Zval.
    #[must_use]
    pub fn get_type(&self) -> DataType {
        DataType::from(u32::from(unsafe { self.u1.v.type_ }))
    }

    /// Returns true if the zval is a long, false otherwise.
    #[must_use]
    pub fn is_long(&self) -> bool {
        self.get_type() == DataType::Long
    }

    /// Returns true if the zval is null, false otherwise.
    #[must_use]
    pub fn is_null(&self) -> bool {
        self.get_type() == DataType::Null
    }

    /// Returns true if the zval is true, false otherwise.
    #[must_use]
    pub fn is_true(&self) -> bool {
        self.get_type() == DataType::True
    }

    /// Returns true if the zval is false, false otherwise.
    #[must_use]
    pub fn is_false(&self) -> bool {
        self.get_type() == DataType::False
    }

    /// Returns true if the zval is a bool, false otherwise.
    #[must_use]
    pub fn is_bool(&self) -> bool {
        self.is_true() || self.is_false()
    }

    /// Returns true if the zval is a double, false otherwise.
    #[must_use]
    pub fn is_double(&self) -> bool {
        self.get_type() == DataType::Double
    }

    /// Returns true if the zval is a string, false otherwise.
    #[must_use]
    pub fn is_string(&self) -> bool {
        self.get_type() == DataType::String
    }

    /// Returns true if the zval is a resource, false otherwise.
    #[must_use]
    pub fn is_resource(&self) -> bool {
        self.get_type() == DataType::Resource
    }

    /// Returns true if the zval is an array, false otherwise.
    #[must_use]
    pub fn is_array(&self) -> bool {
        self.get_type() == DataType::Array
    }

    /// Returns true if the zval is an object, false otherwise.
    #[must_use]
    pub fn is_object(&self) -> bool {
        matches!(self.get_type(), DataType::Object(_))
    }

    /// Returns true if the zval is a reference, false otherwise.
    #[must_use]
    pub fn is_reference(&self) -> bool {
        self.get_type() == DataType::Reference
    }

    /// Returns true if the zval is a reference, false otherwise.
    #[must_use]
    pub fn is_indirect(&self) -> bool {
        self.get_type() == DataType::Indirect
    }

    /// Returns true if the zval is callable, false otherwise.
    #[must_use]
    pub fn is_callable(&self) -> bool {
        let ptr: *const Self = self;
        unsafe { zend_is_callable(ptr.cast_mut(), 0, std::ptr::null_mut()) }
    }

    /// Checks if the zval is identical to another one.
    /// This works like `===` in php.
    ///
    /// # Parameters
    ///
    /// * `other` - The the zval to check identity against.
    #[must_use]
    pub fn is_identical(&self, other: &Self) -> bool {
        let self_p: *const Self = self;
        let other_p: *const Self = other;
        unsafe { zend_is_identical(self_p.cast_mut(), other_p.cast_mut()) }
    }

    /// Returns true if the zval is traversable, false otherwise.
    #[must_use]
    pub fn is_traversable(&self) -> bool {
        match self.object() {
            None => false,
            Some(obj) => obj.is_traversable(),
        }
    }

    /// Returns true if the zval is iterable (array or traversable), false
    /// otherwise.
    #[must_use]
    pub fn is_iterable(&self) -> bool {
        let ptr: *const Self = self;
        unsafe { zend_is_iterable(ptr.cast_mut()) }
    }

    /// Returns true if the zval contains a pointer, false otherwise.
    #[must_use]
    pub fn is_ptr(&self) -> bool {
        self.get_type() == DataType::Ptr
    }

    /// Returns true if the zval is a scalar value (integer, float, string, or
    /// bool), false otherwise.
    ///
    /// This is equivalent to PHP's `is_scalar()` function.
    #[must_use]
    pub fn is_scalar(&self) -> bool {
        matches!(
            self.get_type(),
            DataType::Long | DataType::Double | DataType::String | DataType::True | DataType::False
        )
    }

    // =========================================================================
    // Type Coercion Methods
    // =========================================================================
    //
    // These methods convert the zval's value to a different type following
    // PHP's type coercion rules. Unlike the mutating `coerce_into_*` methods
    // in some implementations, these are pure functions that return a new value.

    /// Coerces the value to a boolean following PHP's type coercion rules.
    ///
    /// This uses PHP's internal `zend_is_true` function to determine the
    /// boolean value, which handles all PHP types correctly:
    /// - `null` → `false`
    /// - `false` → `false`, `true` → `true`
    /// - `0`, `0.0`, `""`, `"0"` → `false`
    /// - Empty arrays → `false`
    /// - Everything else → `true`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::Zval;
    ///
    /// let mut zv = Zval::new();
    /// zv.set_long(0);
    /// assert_eq!(zv.coerce_to_bool(), false);
    ///
    /// zv.set_long(42);
    /// assert_eq!(zv.coerce_to_bool(), true);
    /// ```
    #[must_use]
    pub fn coerce_to_bool(&self) -> bool {
        cfg_if! {
            if #[cfg(php84)] {
                let ptr: *const Self = self;
                unsafe { zend_is_true(ptr) }
            } else {
                // Pre-PHP 8.4: zend_is_true takes *mut and returns c_int
                let ptr = self as *const Self as *mut Self;
                unsafe { zend_is_true(ptr) != 0 }
            }
        }
    }

    /// Coerces the value to a string following PHP's type coercion rules.
    ///
    /// This is a Rust implementation that matches PHP's behavior. Returns `None`
    /// for types that cannot be meaningfully converted to strings (arrays,
    /// resources, objects without `__toString`).
    ///
    /// Conversion rules:
    /// - Strings → returned as-is
    /// - Integers → decimal string representation
    /// - Floats → string representation (uses uppercase `E` for scientific
    ///   notation when exponent >= 14 or <= -5, matching PHP)
    /// - `true` → `"1"`, `false` → `""`
    /// - `null` → `""`
    /// - `INF` → `"INF"`, `-INF` → `"-INF"`, `NAN` → `"NAN"` (uppercase)
    /// - Objects with `__toString()` → result of calling `__toString()`
    /// - Arrays, resources, objects without `__toString()` → `None`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::Zval;
    ///
    /// let mut zv = Zval::new();
    /// zv.set_long(42);
    /// assert_eq!(zv.coerce_to_string(), Some("42".to_string()));
    ///
    /// zv.set_bool(true);
    /// assert_eq!(zv.coerce_to_string(), Some("1".to_string()));
    /// ```
    #[must_use]
    pub fn coerce_to_string(&self) -> Option<String> {
        // Already a string
        if let Some(s) = self.str() {
            return Some(s.to_string());
        }

        // Boolean
        if let Some(b) = self.bool() {
            return Some(if b { "1".to_string() } else { String::new() });
        }

        // Null
        if self.is_null() {
            return Some(String::new());
        }

        // Integer
        if let Some(l) = self.long() {
            return Some(l.to_string());
        }

        // Float
        if let Some(d) = self.double() {
            // PHP uses uppercase for special float values
            if d.is_nan() {
                return Some("NAN".to_string());
            }
            if d.is_infinite() {
                return Some(if d.is_sign_positive() {
                    "INF".to_string()
                } else {
                    "-INF".to_string()
                });
            }
            // PHP uses a format similar to printf's %G with ~14 digits precision
            // and uppercase E for scientific notation
            return Some(php_float_to_string(d));
        }

        // Object with __toString
        if let Some(obj) = self.object()
            && let Ok(result) = obj.try_call_method("__toString", vec![])
        {
            return result.str().map(ToString::to_string);
        }

        // Arrays, resources, and objects without __toString cannot be converted
        None
    }

    /// Coerces the value to an integer following PHP's type coercion rules.
    ///
    /// This is a Rust implementation that matches PHP's behavior. Returns `None`
    /// for types that cannot be meaningfully converted to integers (arrays,
    /// resources, objects).
    ///
    /// Conversion rules:
    /// - Integers → returned as-is
    /// - Floats → truncated toward zero
    /// - `true` → `1`, `false` → `0`
    /// - `null` → `0`
    /// - Strings → parsed as integer (leading numeric portion only; stops at
    ///   first non-digit; returns 0 if non-numeric; saturates on overflow)
    /// - Arrays, resources, objects → `None`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::Zval;
    ///
    /// let mut zv = Zval::new();
    /// zv.set_string("42abc", false);
    /// assert_eq!(zv.coerce_to_long(), Some(42));
    ///
    /// zv.set_double(3.7);
    /// assert_eq!(zv.coerce_to_long(), Some(3));
    /// ```
    #[must_use]
    pub fn coerce_to_long(&self) -> Option<ZendLong> {
        // Already an integer
        if let Some(l) = self.long() {
            return Some(l);
        }

        // Boolean
        if let Some(b) = self.bool() {
            return Some(ZendLong::from(b));
        }

        // Null
        if self.is_null() {
            return Some(0);
        }

        // Float - truncate toward zero
        if let Some(d) = self.double() {
            #[allow(clippy::cast_possible_truncation)]
            return Some(d as ZendLong);
        }

        // String - parse leading numeric portion
        if let Some(s) = self.str() {
            return Some(parse_long_from_str(s));
        }

        // Arrays, resources, objects cannot be converted
        None
    }

    /// Coerces the value to a float following PHP's type coercion rules.
    ///
    /// This is a Rust implementation that matches PHP's behavior. Returns `None`
    /// for types that cannot be meaningfully converted to floats (arrays,
    /// resources, objects).
    ///
    /// Conversion rules:
    /// - Floats → returned as-is
    /// - Integers → converted to float
    /// - `true` → `1.0`, `false` → `0.0`
    /// - `null` → `0.0`
    /// - Strings → parsed as float (leading numeric portion including decimal
    ///   point and scientific notation; returns 0.0 if non-numeric)
    /// - Arrays, resources, objects → `None`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::Zval;
    ///
    /// let mut zv = Zval::new();
    /// zv.set_string("3.14abc", false);
    /// assert_eq!(zv.coerce_to_double(), Some(3.14));
    ///
    /// zv.set_long(42);
    /// assert_eq!(zv.coerce_to_double(), Some(42.0));
    /// ```
    #[must_use]
    pub fn coerce_to_double(&self) -> Option<f64> {
        // Already a float
        if let Some(d) = self.double() {
            return Some(d);
        }

        // Integer
        if let Some(l) = self.long() {
            #[allow(clippy::cast_precision_loss)]
            return Some(l as f64);
        }

        // Boolean
        if let Some(b) = self.bool() {
            return Some(if b { 1.0 } else { 0.0 });
        }

        // Null
        if self.is_null() {
            return Some(0.0);
        }

        // String - parse leading numeric portion
        if let Some(s) = self.str() {
            return Some(parse_double_from_str(s));
        }

        // Arrays, resources, objects cannot be converted
        None
    }

    /// Sets the value of the zval as a string. Returns nothing in a result when
    /// successful.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to set the zval as.
    /// * `persistent` - Whether the string should persist between requests.
    ///
    /// # Persistent Strings
    ///
    /// When `persistent` is `true`, the string is allocated from PHP's
    /// persistent heap (using `malloc`) rather than the request-bound heap.
    /// This is typically used for strings that need to survive across multiple
    /// PHP requests, such as class names, function names, or module-level data.
    ///
    /// **Important:** The string will still be freed when the Zval is dropped.
    /// The `persistent` flag only affects which memory allocator is used. If
    /// you need a string to outlive the Zval, consider using
    /// [`std::mem::forget`] on the Zval or storing the string elsewhere.
    ///
    /// For most use cases (return values, function arguments, temporary
    /// storage), you should use `persistent: false`.
    ///
    /// # Errors
    ///
    /// Never returns an error.
    // TODO: Check if we can drop the result here.
    pub fn set_string(&mut self, val: &str, persistent: bool) -> Result<()> {
        self.set_zend_string(ZendStr::new(val, persistent));
        Ok(())
    }

    /// Sets the value of the zval as a Zend string.
    ///
    /// The Zval takes ownership of the string. When the Zval is dropped,
    /// the string will be released.
    ///
    /// # Parameters
    ///
    /// * `val` - String content.
    pub fn set_zend_string(&mut self, val: ZBox<ZendStr>) {
        self.change_type(ZvalTypeFlags::StringEx);
        self.value.str_ = val.into_raw();
    }

    /// Sets the value of the zval as a binary string, which is represented in
    /// Rust as a vector.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to set the zval as.
    pub fn set_binary<T: Pack>(&mut self, val: Vec<T>) {
        self.change_type(ZvalTypeFlags::StringEx);
        let ptr = T::pack_into(val);
        self.value.str_ = ptr;
    }

    /// Sets the value of the zval as an interned string. Returns nothing in a
    /// result when successful.
    ///
    /// Interned strings are stored once and are immutable. PHP stores them in
    /// an internal hashtable. Unlike regular strings, interned strings are not
    /// reference counted and should not be freed by `zval_ptr_dtor`.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to set the zval as.
    /// * `persistent` - Whether the string should persist between requests.
    ///
    /// # Errors
    ///
    /// Never returns an error.
    // TODO: Check if we can drop the result here.
    pub fn set_interned_string(&mut self, val: &str, persistent: bool) -> Result<()> {
        // Use InternedStringEx (without RefCounted) because interned strings
        // should not have their refcount modified by zval_ptr_dtor.
        self.change_type(ZvalTypeFlags::InternedStringEx);
        self.value.str_ = ZendStr::new_interned(val, persistent).into_raw();
        Ok(())
    }

    /// Sets the value of the zval as a long.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to set the zval as.
    pub fn set_long<T: Into<ZendLong>>(&mut self, val: T) {
        self.internal_set_long(val.into());
    }

    fn internal_set_long(&mut self, val: ZendLong) {
        self.change_type(ZvalTypeFlags::Long);
        self.value.lval = val;
    }

    /// Sets the value of the zval as a double.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to set the zval as.
    pub fn set_double<T: Into<f64>>(&mut self, val: T) {
        self.internal_set_double(val.into());
    }

    fn internal_set_double(&mut self, val: f64) {
        self.change_type(ZvalTypeFlags::Double);
        self.value.dval = val;
    }

    /// Sets the value of the zval as a boolean.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to set the zval as.
    pub fn set_bool<T: Into<bool>>(&mut self, val: T) {
        self.internal_set_bool(val.into());
    }

    fn internal_set_bool(&mut self, val: bool) {
        self.change_type(if val {
            ZvalTypeFlags::True
        } else {
            ZvalTypeFlags::False
        });
    }

    /// Sets the value of the zval as null.
    ///
    /// This is the default of a zval.
    pub fn set_null(&mut self) {
        self.change_type(ZvalTypeFlags::Null);
    }

    /// Sets the value of the zval as a resource.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to set the zval as.
    pub fn set_resource(&mut self, val: *mut zend_resource) {
        self.change_type(ZvalTypeFlags::ResourceEx);
        self.value.res = val;
    }

    /// Sets the value of the zval as a reference to an object.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to set the zval as.
    pub fn set_object(&mut self, val: &mut ZendObject) {
        self.change_type(ZvalTypeFlags::ObjectEx);
        val.inc_count(); // TODO(david): not sure if this is needed :/
        self.value.obj = ptr::from_ref(val).cast_mut();
    }

    /// Sets the value of the zval as an array. Returns nothing in a result on
    /// success.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to set the zval as.
    ///
    /// # Errors
    ///
    /// * Returns an error if the conversion to a hashtable fails.
    pub fn set_array<T: TryInto<ZBox<ZendHashTable>, Error = Error>>(
        &mut self,
        val: T,
    ) -> Result<()> {
        self.set_hashtable(val.try_into()?);
        Ok(())
    }

    /// Sets the value of the zval as an array. Returns nothing in a result on
    /// success.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to set the zval as.
    pub fn set_hashtable(&mut self, val: ZBox<ZendHashTable>) {
        // Handle immutable shared arrays (e.g., the empty array) similar to
        // ZVAL_EMPTY_ARRAY. Immutable arrays should not be reference counted.
        let type_info = if val.is_immutable() {
            ZvalTypeFlags::Array
        } else {
            ZvalTypeFlags::ArrayEx
        };
        self.change_type(type_info);
        self.value.arr = val.into_raw();
    }

    /// Sets the value of the zval as a pointer.
    ///
    /// # Parameters
    ///
    /// * `ptr` - The pointer to set the zval as.
    pub fn set_ptr<T>(&mut self, ptr: *mut T) {
        self.u1.type_info = ZvalTypeFlags::Ptr.bits();
        self.value.ptr = ptr.cast::<c_void>();
    }

    /// Used to drop the Zval but keep the value of the zval intact.
    ///
    /// This is important when copying the value of the zval, as the actual
    /// value will not be copied, but the pointer to the value (string for
    /// example) will be copied.
    pub(crate) fn release(mut self) {
        // NOTE(david): don't use `change_type` here as we are wanting to keep the
        // contents intact.
        self.u1.type_info = ZvalTypeFlags::Null.bits();
    }

    /// Changes the type of the zval, freeing the current contents when
    /// applicable.
    ///
    /// # Parameters
    ///
    /// * `ty` - The new type of the zval.
    fn change_type(&mut self, ty: ZvalTypeFlags) {
        // SAFETY: we have exclusive mutable access to this zval so can free the
        // contents.
        //
        // For strings, we use zend_string_release directly instead of zval_ptr_dtor
        // to correctly handle persistent strings. zend_string_release properly checks
        // the IS_STR_PERSISTENT flag and uses the correct deallocator (free vs efree).
        // This fixes heap corruption issues when dropping Zvals containing persistent
        // strings (see issue #424).
        //
        // For simple types (Long, Double, Bool, Null, Undef, True, False), we don't
        // need to call zval_ptr_dtor because they don't have heap-allocated data.
        // This is important for cargo-php stub generation where zval_ptr_dtor is
        // stubbed as a null pointer - calling it would crash.
        if self.is_string() {
            unsafe {
                if let Some(str_ptr) = self.value.str_.as_mut() {
                    ext_php_rs_zend_string_release(str_ptr);
                }
            }
        } else if self.is_array()
            || self.is_object()
            || self.is_resource()
            || self.is_reference()
            || self.get_type() == DataType::ConstantExpression
        {
            // Only call zval_ptr_dtor for reference-counted types
            unsafe { zval_ptr_dtor(self) };
        }
        // Simple types (Long, Double, Bool, Null, etc.) need no cleanup
        self.u1.type_info = ty.bits();
    }

    /// Extracts some type from a `Zval`.
    ///
    /// This is a wrapper function around `TryFrom`.
    #[must_use]
    pub fn extract<'a, T>(&'a self) -> Option<T>
    where
        T: FromZval<'a>,
    {
        FromZval::from_zval(self)
    }

    /// Creates a shallow clone of the [`Zval`].
    ///
    /// This copies the contents of the [`Zval`], and increments the reference
    /// counter of the underlying value (if it is reference counted).
    ///
    /// For example, if the zval contains a long, it will simply copy the value.
    /// However, if the zval contains an object, the new zval will point to the
    /// same object, and the objects reference counter will be incremented.
    ///
    /// # Returns
    ///
    /// The cloned zval.
    #[must_use]
    pub fn shallow_clone(&self) -> Zval {
        let mut new = Zval::new();
        new.u1 = self.u1;
        new.value = self.value;

        // SAFETY: `u1` union is only used for easier bitmasking. It is valid to read
        // from either of the variants.
        //
        // SAFETY: If the value if refcounted (`self.u1.type_info & Z_TYPE_FLAGS_MASK`)
        // then it is valid to dereference `self.value.counted`.
        unsafe {
            let flags = ZvalTypeFlags::from_bits_retain(self.u1.type_info);
            if flags.contains(ZvalTypeFlags::RefCounted) {
                (*self.value.counted).gc.refcount += 1;
            }
        }

        new
    }
}

impl Debug for Zval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("Zval");
        let ty = self.get_type();
        dbg.field("type", &ty);

        macro_rules! field {
            ($value: expr) => {
                dbg.field("val", &$value)
            };
        }

        match ty {
            DataType::Undef | DataType::Null | DataType::ConstantExpression | DataType::Void => {
                field!(Option::<()>::None)
            }
            DataType::False => field!(false),
            DataType::True => field!(true),
            DataType::Long => field!(self.long()),
            DataType::Double => field!(self.double()),
            DataType::String | DataType::Mixed | DataType::Callable => field!(self.string()),
            DataType::Array => field!(self.array()),
            DataType::Object(_) => field!(self.object()),
            DataType::Resource => field!(self.resource()),
            DataType::Reference => field!(self.reference()),
            DataType::Bool => field!(self.bool()),
            DataType::Indirect => field!(self.indirect()),
            DataType::Iterable => field!(self.iterable()),
            // SAFETY: We are not accessing the pointer.
            DataType::Ptr => field!(unsafe { self.ptr::<c_void>() }),
        };

        dbg.finish()
    }
}

impl Drop for Zval {
    fn drop(&mut self) {
        self.change_type(ZvalTypeFlags::Null);
    }
}

impl Default for Zval {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoZval for Zval {
    const TYPE: DataType = DataType::Mixed;
    const NULLABLE: bool = true;

    fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
        *zv = self;
        Ok(())
    }
}

impl<'a> FromZval<'a> for &'a Zval {
    const TYPE: DataType = DataType::Mixed;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        Some(zval)
    }
}

impl<'a> FromZvalMut<'a> for &'a mut Zval {
    const TYPE: DataType = DataType::Mixed;

    fn from_zval_mut(zval: &'a mut Zval) -> Option<Self> {
        Some(zval)
    }
}

/// Formats a float to a string following PHP's type coercion rules.
///
/// This is a Rust implementation that matches PHP's behavior. PHP uses a format
/// similar to printf's `%G` with 14 significant digits:
/// - Uses scientific notation when exponent >= 14 or <= -5
/// - Uses uppercase 'E' for the exponent (e.g., "1.0E+52")
/// - Removes trailing zeros after the decimal point
fn php_float_to_string(d: f64) -> String {
    // Use Rust's %G-like formatting: format with precision, uppercase E
    // PHP uses ~14 significant digits
    let formatted = format!("{d:.14E}");

    // Parse the formatted string to clean it up
    if let Some(e_pos) = formatted.find('E') {
        let mantissa_part = &formatted[..e_pos];
        let exp_part = &formatted[e_pos..];

        // Parse exponent to decide format
        let exp: i32 = exp_part[1..].parse().unwrap_or(0);

        // PHP uses scientific notation when exponent >= 14 or <= -5
        if exp >= 14 || exp <= -5 {
            // Keep scientific notation, but clean up mantissa
            let mantissa = mantissa_part.trim_end_matches('0').trim_end_matches('.');

            // Ensure mantissa has at least one decimal digit if there's a decimal point
            let mantissa = if mantissa.contains('.') && mantissa.ends_with('.') {
                format!("{mantissa}0")
            } else if !mantissa.contains('.') {
                format!("{mantissa}.0")
            } else {
                mantissa.to_string()
            };

            // Format exponent with sign
            if exp >= 0 {
                format!("{mantissa}E+{exp}")
            } else {
                format!("{mantissa}E{exp}")
            }
        } else {
            // Use decimal notation
            let s = d.to_string();
            // Clean up trailing zeros
            if s.contains('.') {
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            } else {
                s
            }
        }
    } else {
        formatted
    }
}

/// Parses an integer from a string following PHP's type coercion rules.
///
/// This is a Rust implementation that matches PHP's behavior. It extracts the
/// leading numeric portion of a string:
/// - `"42"` → 42
/// - `"42abc"` → 42
/// - `"  42"` → 42 (leading whitespace is skipped)
/// - `"-42"` → -42
/// - `"abc"` → 0
/// - `""` → 0
/// - Hexadecimal (`"0xFF"`) → 0 (not interpreted, stops at 'x')
/// - Octal (`"010"`) → 10 (not interpreted as octal)
/// - Overflow saturates to `ZendLong::MAX` or `ZendLong::MIN`
fn parse_long_from_str(s: &str) -> ZendLong {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return 0;
    }

    let mut i = 0;
    let mut is_negative = false;

    // Handle optional sign
    if bytes[i] == b'-' || bytes[i] == b'+' {
        is_negative = bytes[i] == b'-';
        i += 1;
    }

    let digits_start = i;

    // Collect digits
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }

    // No digits found
    if i == digits_start {
        return 0;
    }

    // Parse the slice, saturating on overflow (matching PHP behavior)
    match s[..i].parse::<ZendLong>() {
        Ok(n) => n,
        Err(_) => {
            // Overflow: saturate to max or min depending on sign
            if is_negative {
                ZendLong::MIN
            } else {
                ZendLong::MAX
            }
        }
    }
}

/// Parses a float from a string following PHP's type coercion rules.
///
/// This is a Rust implementation that matches PHP's behavior. It extracts the
/// leading numeric portion of a string:
/// - `"3.14"` → 3.14
/// - `"3.14abc"` → 3.14
/// - `"  3.14"` → 3.14 (leading whitespace is skipped)
/// - `"-3.14"` → -3.14
/// - `"1e10"` → 1e10 (scientific notation supported)
/// - `".5"` → 0.5 (leading decimal point)
/// - `"5."` → 5.0 (trailing decimal point)
/// - `"abc"` → 0.0
/// - `""` → 0.0
fn parse_double_from_str(s: &str) -> f64 {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return 0.0;
    }

    let mut i = 0;
    let mut has_decimal = false;
    let mut has_exponent = false;
    let mut has_digits = false;

    // Handle optional sign
    if bytes[i] == b'-' || bytes[i] == b'+' {
        i += 1;
    }

    // Collect digits, decimal point, and exponent
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_digit() {
            has_digits = true;
            i += 1;
        } else if c == b'.' && !has_decimal && !has_exponent {
            has_decimal = true;
            i += 1;
        } else if (c == b'e' || c == b'E') && !has_exponent && has_digits {
            has_exponent = true;
            i += 1;
            // Handle optional sign after exponent
            if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
                i += 1;
            }
        } else {
            break;
        }
    }

    // Parse the slice or return 0.0
    s[..i].parse().unwrap_or(0.0)
}

#[cfg(test)]
#[cfg(feature = "embed")]
#[allow(clippy::unwrap_used, clippy::approx_constant)]
mod tests {
    use super::*;
    use crate::embed::Embed;

    #[test]
    fn test_zval_null() {
        Embed::run(|| {
            let zval = Zval::null();
            assert!(zval.is_null());
        });
    }

    #[test]
    fn test_is_scalar() {
        Embed::run(|| {
            // Test scalar types - should return true
            let mut zval_long = Zval::new();
            zval_long.set_long(42);
            assert!(zval_long.is_scalar());

            let mut zval_double = Zval::new();
            zval_double.set_double(1.5);
            assert!(zval_double.is_scalar());

            let mut zval_true = Zval::new();
            zval_true.set_bool(true);
            assert!(zval_true.is_scalar());

            let mut zval_false = Zval::new();
            zval_false.set_bool(false);
            assert!(zval_false.is_scalar());

            let mut zval_string = Zval::new();
            zval_string
                .set_string("hello", false)
                .expect("set_string should succeed");
            assert!(zval_string.is_scalar());

            // Test non-scalar types - should return false
            let zval_null = Zval::null();
            assert!(!zval_null.is_scalar());

            let zval_array = Zval::new_array();
            assert!(!zval_array.is_scalar());
        });
    }

    #[test]
    fn test_coerce_to_bool() {
        Embed::run(|| {
            let mut zv = Zval::new();

            // === Truthy values ===
            // Integers
            zv.set_long(42);
            assert!(zv.coerce_to_bool(), "(bool)42 should be true");

            zv.set_long(1);
            assert!(zv.coerce_to_bool(), "(bool)1 should be true");

            zv.set_long(-1);
            assert!(zv.coerce_to_bool(), "(bool)-1 should be true");

            // Floats
            zv.set_double(0.1);
            assert!(zv.coerce_to_bool(), "(bool)0.1 should be true");

            zv.set_double(-0.1);
            assert!(zv.coerce_to_bool(), "(bool)-0.1 should be true");

            // Strings - non-empty and not "0"
            zv.set_string("hello", false).unwrap();
            assert!(zv.coerce_to_bool(), "(bool)'hello' should be true");

            zv.set_string("1", false).unwrap();
            assert!(zv.coerce_to_bool(), "(bool)'1' should be true");

            // PHP: (bool)"false" is true (non-empty string)
            zv.set_string("false", false).unwrap();
            assert!(zv.coerce_to_bool(), "(bool)'false' should be true");

            // PHP: (bool)"true" is true
            zv.set_string("true", false).unwrap();
            assert!(zv.coerce_to_bool(), "(bool)'true' should be true");

            // PHP: (bool)" " (space) is true
            zv.set_string(" ", false).unwrap();
            assert!(zv.coerce_to_bool(), "(bool)' ' should be true");

            // PHP: (bool)"00" is true (only exactly "0" is false)
            zv.set_string("00", false).unwrap();
            assert!(zv.coerce_to_bool(), "(bool)'00' should be true");

            zv.set_bool(true);
            assert!(zv.coerce_to_bool(), "(bool)true should be true");

            // === Falsy values ===
            // Integer zero
            zv.set_long(0);
            assert!(!zv.coerce_to_bool(), "(bool)0 should be false");

            // Float zero (both positive and negative)
            zv.set_double(0.0);
            assert!(!zv.coerce_to_bool(), "(bool)0.0 should be false");

            zv.set_double(-0.0);
            assert!(!zv.coerce_to_bool(), "(bool)-0.0 should be false");

            // Empty string
            zv.set_string("", false).unwrap();
            assert!(!zv.coerce_to_bool(), "(bool)'' should be false");

            // String "0" - the only non-empty falsy string
            zv.set_string("0", false).unwrap();
            assert!(!zv.coerce_to_bool(), "(bool)'0' should be false");

            zv.set_bool(false);
            assert!(!zv.coerce_to_bool(), "(bool)false should be false");

            // Null
            let null_zv = Zval::null();
            assert!(!null_zv.coerce_to_bool(), "(bool)null should be false");

            // Empty array
            let empty_array = Zval::new_array();
            assert!(!empty_array.coerce_to_bool(), "(bool)[] should be false");
        });
    }

    #[test]
    fn test_coerce_to_string() {
        Embed::run(|| {
            let mut zv = Zval::new();

            // === Integer to string ===
            zv.set_long(42);
            assert_eq!(zv.coerce_to_string(), Some("42".to_string()));

            zv.set_long(-123);
            assert_eq!(zv.coerce_to_string(), Some("-123".to_string()));

            zv.set_long(0);
            assert_eq!(zv.coerce_to_string(), Some("0".to_string()));

            // === Float to string ===
            zv.set_double(3.14);
            assert_eq!(zv.coerce_to_string(), Some("3.14".to_string()));

            zv.set_double(-3.14);
            assert_eq!(zv.coerce_to_string(), Some("-3.14".to_string()));

            zv.set_double(0.0);
            assert_eq!(zv.coerce_to_string(), Some("0".to_string()));

            zv.set_double(1.0);
            assert_eq!(zv.coerce_to_string(), Some("1".to_string()));

            zv.set_double(1.5);
            assert_eq!(zv.coerce_to_string(), Some("1.5".to_string()));

            zv.set_double(100.0);
            assert_eq!(zv.coerce_to_string(), Some("100".to_string()));

            // Special float values (uppercase like PHP)
            zv.set_double(f64::INFINITY);
            assert_eq!(zv.coerce_to_string(), Some("INF".to_string()));

            zv.set_double(f64::NEG_INFINITY);
            assert_eq!(zv.coerce_to_string(), Some("-INF".to_string()));

            zv.set_double(f64::NAN);
            assert_eq!(zv.coerce_to_string(), Some("NAN".to_string()));

            // Scientific notation thresholds (PHP uses E notation at exp >= 14 or <= -5)
            // PHP: (string)1e13 -> "10000000000000"
            zv.set_double(1e13);
            assert_eq!(zv.coerce_to_string(), Some("10000000000000".to_string()));

            // PHP: (string)1e14 -> "1.0E+14"
            zv.set_double(1e14);
            assert_eq!(zv.coerce_to_string(), Some("1.0E+14".to_string()));

            // PHP: (string)1e52 -> "1.0E+52"
            zv.set_double(1e52);
            assert_eq!(zv.coerce_to_string(), Some("1.0E+52".to_string()));

            // PHP: (string)0.0001 -> "0.0001"
            zv.set_double(0.0001);
            assert_eq!(zv.coerce_to_string(), Some("0.0001".to_string()));

            // PHP: (string)1e-5 -> "1.0E-5"
            zv.set_double(1e-5);
            assert_eq!(zv.coerce_to_string(), Some("1.0E-5".to_string()));

            // Negative scientific notation
            // PHP: (string)(-1e52) -> "-1.0E+52"
            zv.set_double(-1e52);
            assert_eq!(zv.coerce_to_string(), Some("-1.0E+52".to_string()));

            // PHP: (string)(-1e-10) -> "-1.0E-10"
            zv.set_double(-1e-10);
            assert_eq!(zv.coerce_to_string(), Some("-1.0E-10".to_string()));

            // === Boolean to string ===
            // PHP: (string)true -> "1"
            zv.set_bool(true);
            assert_eq!(zv.coerce_to_string(), Some("1".to_string()));

            // PHP: (string)false -> ""
            zv.set_bool(false);
            assert_eq!(zv.coerce_to_string(), Some(String::new()));

            // === Null to string ===
            // PHP: (string)null -> ""
            let null_zv = Zval::null();
            assert_eq!(null_zv.coerce_to_string(), Some(String::new()));

            // === String unchanged ===
            zv.set_string("hello", false).unwrap();
            assert_eq!(zv.coerce_to_string(), Some("hello".to_string()));

            // === Array cannot be converted ===
            let arr_zv = Zval::new_array();
            assert_eq!(arr_zv.coerce_to_string(), None);
        });
    }

    #[test]
    fn test_coerce_to_long() {
        Embed::run(|| {
            let mut zv = Zval::new();

            // === Integer unchanged ===
            zv.set_long(42);
            assert_eq!(zv.coerce_to_long(), Some(42));

            zv.set_long(-42);
            assert_eq!(zv.coerce_to_long(), Some(-42));

            zv.set_long(0);
            assert_eq!(zv.coerce_to_long(), Some(0));

            // === Float truncated (towards zero) ===
            // PHP: (int)3.14 -> 3
            zv.set_double(3.14);
            assert_eq!(zv.coerce_to_long(), Some(3));

            // PHP: (int)3.99 -> 3
            zv.set_double(3.99);
            assert_eq!(zv.coerce_to_long(), Some(3));

            // PHP: (int)-3.14 -> -3
            zv.set_double(-3.14);
            assert_eq!(zv.coerce_to_long(), Some(-3));

            // PHP: (int)-3.99 -> -3
            zv.set_double(-3.99);
            assert_eq!(zv.coerce_to_long(), Some(-3));

            // PHP: (int)0.0 -> 0
            zv.set_double(0.0);
            assert_eq!(zv.coerce_to_long(), Some(0));

            // PHP: (int)-0.0 -> 0
            zv.set_double(-0.0);
            assert_eq!(zv.coerce_to_long(), Some(0));

            // === Boolean to integer ===
            // PHP: (int)true -> 1
            zv.set_bool(true);
            assert_eq!(zv.coerce_to_long(), Some(1));

            // PHP: (int)false -> 0
            zv.set_bool(false);
            assert_eq!(zv.coerce_to_long(), Some(0));

            // === Null to integer ===
            // PHP: (int)null -> 0
            let null_zv = Zval::null();
            assert_eq!(null_zv.coerce_to_long(), Some(0));

            // === String to integer (leading numeric portion) ===
            // PHP: (int)"123" -> 123
            zv.set_string("123", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(123));

            // PHP: (int)"  123" -> 123
            zv.set_string("  123", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(123));

            // PHP: (int)"123abc" -> 123
            zv.set_string("123abc", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(123));

            // PHP: (int)"abc123" -> 0
            zv.set_string("abc123", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(0));

            // PHP: (int)"" -> 0
            zv.set_string("", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(0));

            // PHP: (int)"0" -> 0
            zv.set_string("0", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(0));

            // PHP: (int)"-0" -> 0
            zv.set_string("-0", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(0));

            // PHP: (int)"+123" -> 123
            zv.set_string("+123", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(123));

            // PHP: (int)"   -456" -> -456
            zv.set_string("   -456", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(-456));

            // PHP: (int)"3.14" -> 3 (stops at decimal point)
            zv.set_string("3.14", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(3));

            // PHP: (int)"3.99" -> 3
            zv.set_string("3.99", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(3));

            // PHP: (int)"-3.99" -> -3
            zv.set_string("-3.99", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(-3));

            // PHP: (int)"abc" -> 0
            zv.set_string("abc", false).unwrap();
            assert_eq!(zv.coerce_to_long(), Some(0));

            // === Array cannot be converted ===
            let arr_zv = Zval::new_array();
            assert_eq!(arr_zv.coerce_to_long(), None);
        });
    }

    #[test]
    fn test_coerce_to_double() {
        Embed::run(|| {
            let mut zv = Zval::new();

            // === Float unchanged ===
            zv.set_double(3.14);
            assert!((zv.coerce_to_double().unwrap() - 3.14).abs() < f64::EPSILON);

            zv.set_double(-3.14);
            assert!((zv.coerce_to_double().unwrap() - (-3.14)).abs() < f64::EPSILON);

            zv.set_double(0.0);
            assert!((zv.coerce_to_double().unwrap() - 0.0).abs() < f64::EPSILON);

            // === Integer to float ===
            // PHP: (float)42 -> 42.0
            zv.set_long(42);
            assert!((zv.coerce_to_double().unwrap() - 42.0).abs() < f64::EPSILON);

            // PHP: (float)-42 -> -42.0
            zv.set_long(-42);
            assert!((zv.coerce_to_double().unwrap() - (-42.0)).abs() < f64::EPSILON);

            // PHP: (float)0 -> 0.0
            zv.set_long(0);
            assert!((zv.coerce_to_double().unwrap() - 0.0).abs() < f64::EPSILON);

            // === Boolean to float ===
            // PHP: (float)true -> 1.0
            zv.set_bool(true);
            assert!((zv.coerce_to_double().unwrap() - 1.0).abs() < f64::EPSILON);

            // PHP: (float)false -> 0.0
            zv.set_bool(false);
            assert!((zv.coerce_to_double().unwrap() - 0.0).abs() < f64::EPSILON);

            // === Null to float ===
            // PHP: (float)null -> 0.0
            let null_zv = Zval::null();
            assert!((null_zv.coerce_to_double().unwrap() - 0.0).abs() < f64::EPSILON);

            // === String to float ===
            // PHP: (float)"3.14" -> 3.14
            zv.set_string("3.14", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 3.14).abs() < f64::EPSILON);

            // PHP: (float)"  3.14" -> 3.14
            zv.set_string("  3.14", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 3.14).abs() < f64::EPSILON);

            // PHP: (float)"3.14abc" -> 3.14
            zv.set_string("3.14abc", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 3.14).abs() < f64::EPSILON);

            // PHP: (float)"abc3.14" -> 0.0
            zv.set_string("abc3.14", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 0.0).abs() < f64::EPSILON);

            // PHP: (float)"" -> 0.0
            zv.set_string("", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 0.0).abs() < f64::EPSILON);

            // PHP: (float)"0" -> 0.0
            zv.set_string("0", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 0.0).abs() < f64::EPSILON);

            // PHP: (float)"0.0" -> 0.0
            zv.set_string("0.0", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 0.0).abs() < f64::EPSILON);

            // PHP: (float)"+3.14" -> 3.14
            zv.set_string("+3.14", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 3.14).abs() < f64::EPSILON);

            // PHP: (float)"   -3.14" -> -3.14
            zv.set_string("   -3.14", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - (-3.14)).abs() < f64::EPSILON);

            // Scientific notation
            // PHP: (float)"1e10" -> 1e10
            zv.set_string("1e10", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 1e10).abs() < 1.0);

            // PHP: (float)"1E10" -> 1e10
            zv.set_string("1E10", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 1e10).abs() < 1.0);

            // PHP: (float)"1.5e-3" -> 0.0015
            zv.set_string("1.5e-3", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 0.0015).abs() < f64::EPSILON);

            // PHP: (float)".5" -> 0.5
            zv.set_string(".5", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 0.5).abs() < f64::EPSILON);

            // PHP: (float)"5." -> 5.0
            zv.set_string("5.", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 5.0).abs() < f64::EPSILON);

            // PHP: (float)"abc" -> 0.0
            zv.set_string("abc", false).unwrap();
            assert!((zv.coerce_to_double().unwrap() - 0.0).abs() < f64::EPSILON);

            // === Array cannot be converted ===
            let arr_zv = Zval::new_array();
            assert_eq!(arr_zv.coerce_to_double(), None);
        });
    }

    #[test]
    fn test_parse_long_from_str() {
        use crate::ffi::zend_long as ZendLong;

        // Basic cases
        assert_eq!(parse_long_from_str("42"), 42);
        assert_eq!(parse_long_from_str("0"), 0);
        assert_eq!(parse_long_from_str("-42"), -42);
        assert_eq!(parse_long_from_str("+42"), 42);

        // Leading/trailing content
        assert_eq!(parse_long_from_str("42abc"), 42);
        assert_eq!(parse_long_from_str("  42"), 42);
        assert_eq!(parse_long_from_str("\t\n42"), 42);
        assert_eq!(parse_long_from_str("  -42"), -42);
        assert_eq!(parse_long_from_str("42.5"), 42); // stops at decimal point

        // Non-numeric strings
        assert_eq!(parse_long_from_str("abc"), 0);
        assert_eq!(parse_long_from_str(""), 0);
        assert_eq!(parse_long_from_str("  "), 0);
        assert_eq!(parse_long_from_str("-"), 0);
        assert_eq!(parse_long_from_str("+"), 0);
        assert_eq!(parse_long_from_str("abc123"), 0);

        // Edge cases matching PHP behavior:
        // - Hexadecimal: PHP's (int)"0xFF" returns 0 (stops at 'x')
        assert_eq!(parse_long_from_str("0xFF"), 0);
        assert_eq!(parse_long_from_str("0x10"), 0);

        // - Octal notation is NOT interpreted, just reads leading digits
        assert_eq!(parse_long_from_str("010"), 10); // not 8

        // - Binary notation is NOT interpreted
        assert_eq!(parse_long_from_str("0b101"), 0);

        // Overflow tests - saturates to max/min
        assert_eq!(
            parse_long_from_str("99999999999999999999999999"),
            ZendLong::MAX
        );
        assert_eq!(
            parse_long_from_str("-99999999999999999999999999"),
            ZendLong::MIN
        );

        // Large but valid numbers
        assert_eq!(parse_long_from_str("9223372036854775807"), ZendLong::MAX); // i64::MAX
        assert_eq!(parse_long_from_str("-9223372036854775808"), ZendLong::MIN); // i64::MIN
    }

    #[test]
    fn test_parse_double_from_str() {
        // Basic cases
        assert!((parse_double_from_str("3.14") - 3.14).abs() < f64::EPSILON);
        assert!((parse_double_from_str("0.0") - 0.0).abs() < f64::EPSILON);
        assert!((parse_double_from_str("-3.14") - (-3.14)).abs() < f64::EPSILON);
        assert!((parse_double_from_str("+3.14") - 3.14).abs() < f64::EPSILON);
        assert!((parse_double_from_str(".5") - 0.5).abs() < f64::EPSILON);
        assert!((parse_double_from_str("5.") - 5.0).abs() < f64::EPSILON);

        // Scientific notation
        assert!((parse_double_from_str("1e10") - 1e10).abs() < 1.0);
        assert!((parse_double_from_str("1E10") - 1e10).abs() < 1.0);
        assert!((parse_double_from_str("1.5e-3") - 1.5e-3).abs() < f64::EPSILON);
        assert!((parse_double_from_str("1.5E+3") - 1500.0).abs() < f64::EPSILON);
        assert!((parse_double_from_str("-1.5e2") - (-150.0)).abs() < f64::EPSILON);

        // Leading/trailing content
        assert!((parse_double_from_str("3.14abc") - 3.14).abs() < f64::EPSILON);
        assert!((parse_double_from_str("  3.14") - 3.14).abs() < f64::EPSILON);
        assert!((parse_double_from_str("\t\n3.14") - 3.14).abs() < f64::EPSILON);
        assert!((parse_double_from_str("1e10xyz") - 1e10).abs() < 1.0);

        // Non-numeric strings
        assert!((parse_double_from_str("abc") - 0.0).abs() < f64::EPSILON);
        assert!((parse_double_from_str("") - 0.0).abs() < f64::EPSILON);
        assert!((parse_double_from_str("  ") - 0.0).abs() < f64::EPSILON);
        assert!((parse_double_from_str("-") - 0.0).abs() < f64::EPSILON);
        assert!((parse_double_from_str("e10") - 0.0).abs() < f64::EPSILON); // no digits before e

        // Integer values
        assert!((parse_double_from_str("42") - 42.0).abs() < f64::EPSILON);
        assert!((parse_double_from_str("-42") - (-42.0)).abs() < f64::EPSILON);

        // Edge cases:
        // - Hexadecimal: not supported, returns 0
        assert!((parse_double_from_str("0xFF") - 0.0).abs() < f64::EPSILON);

        // - Multiple decimal points: stops at second
        assert!((parse_double_from_str("1.2.3") - 1.2).abs() < f64::EPSILON);

        // - Multiple exponents: stops at second e
        assert!((parse_double_from_str("1e2e3") - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_php_float_to_string() {
        // Basic decimal values
        assert_eq!(php_float_to_string(3.14), "3.14");
        assert_eq!(php_float_to_string(42.0), "42");
        assert_eq!(php_float_to_string(-3.14), "-3.14");
        assert_eq!(php_float_to_string(0.0), "0");

        // Large numbers use scientific notation with uppercase E
        // PHP: (string)(float)'999...9' -> "1.0E+52"
        assert_eq!(php_float_to_string(1e52), "1.0E+52");
        assert_eq!(php_float_to_string(1e20), "1.0E+20");
        assert_eq!(php_float_to_string(1e14), "1.0E+14");

        // Very small numbers also use scientific notation (at exp <= -5)
        assert_eq!(php_float_to_string(1e-10), "1.0E-10");
        assert_eq!(php_float_to_string(1e-5), "1.0E-5");
        assert_eq!(php_float_to_string(0.00001), "1.0E-5");

        // Numbers that don't need scientific notation
        assert_eq!(php_float_to_string(1e13), "10000000000000");
        assert_eq!(php_float_to_string(0.001), "0.001");
        assert_eq!(php_float_to_string(0.0001), "0.0001"); // 1e-4 is decimal
    }
}
