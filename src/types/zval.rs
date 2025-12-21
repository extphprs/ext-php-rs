//! The base value in PHP. A Zval can contain any PHP type, and the type that it
//! contains is determined by a property inside the struct. The content of the
//! Zval is stored in a union.

use std::{convert::TryInto, ffi::c_void, fmt::Debug, ptr};

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
        zend_array_dup, zend_is_callable, zend_is_identical, zend_is_iterable, zend_resource,
        zend_value, zval, zval_ptr_dtor,
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

    /// Returns true if the zval is a scalar value (integer, float, string, or bool),
    /// false otherwise.
    ///
    /// This is equivalent to PHP's `is_scalar()` function.
    #[must_use]
    pub fn is_scalar(&self) -> bool {
        matches!(
            self.get_type(),
            DataType::Long | DataType::Double | DataType::String | DataType::True | DataType::False
        )
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
        self.change_type(ZvalTypeFlags::ArrayEx);
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
        if self.is_string() {
            unsafe {
                if let Some(str_ptr) = self.value.str_.as_mut() {
                    ext_php_rs_zend_string_release(str_ptr);
                }
            }
        } else {
            unsafe { zval_ptr_dtor(self) };
        }
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

#[cfg(test)]
#[cfg(feature = "embed")]
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
}
