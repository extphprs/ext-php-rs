//! Represents an object in PHP. Allows for overriding the internal object used
//! by classes, allowing users to store Rust data inside a PHP object.

use std::{
    fmt::Debug,
    mem,
    ops::{Deref, DerefMut},
    os::raw::c_char,
    ptr::{self, NonNull},
};

use crate::{
    boxed::{ZBox, ZBoxable},
    class::RegisteredClass,
    convert::{FromZendObject, FromZendObjectMut, FromZval, FromZvalMut, IntoZval},
    error::{Error, Result},
    ffi::{
        ext_php_rs_zend_object_alloc, ext_php_rs_zend_object_release, object_properties_init,
        zend_object, zend_object_std_init, zend_objects_clone_members,
    },
    flags::DataType,
    types::{ZendObject, Zval},
    zend::ClassEntry,
};

/// Representation of a Zend class object in memory.
#[repr(C)]
#[derive(Debug)]
pub struct ZendClassObject<T> {
    /// The object stored inside the class object.
    pub obj: Option<T>,
    /// The standard zend object.
    pub std: ZendObject,
}

impl<T: RegisteredClass> ZendClassObject<T> {
    /// Creates a new [`ZendClassObject`] of type `T`, where `T` is a
    /// [`RegisteredClass`] in PHP, storing the given value `val` inside the
    /// object.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to store inside the object.
    ///
    /// # Panics
    ///
    /// Panics if memory was unable to be allocated for the new object.
    pub fn new(val: T) -> ZBox<Self> {
        // SAFETY: We are providing a value to initialize the object with.
        unsafe { Self::internal_new(Some(val), None) }
    }

    /// Creates a new [`ZendClassObject`] of type `T`, with an uninitialized
    /// internal object.
    ///
    /// # Safety
    ///
    /// As the object is uninitialized, the caller must ensure the following
    /// until the internal object is initialized:
    ///
    /// * The object is never dereferenced to `T`.
    /// * The [`Clone`] implementation is never called.
    /// * The [`Debug`] implementation is never called.
    ///
    /// If any of these conditions are not met while not initialized, the
    /// corresponding function will panic. Converting the object into its
    /// inner pointer with the [`into_raw`] function is valid, however.
    ///
    /// [`into_raw`]: #method.into_raw
    ///
    /// # Panics
    ///
    /// Panics if memory was unable to be allocated for the new object.
    pub unsafe fn new_uninit(ce: Option<&'static ClassEntry>) -> ZBox<Self> {
        unsafe { Self::internal_new(None, ce) }
    }

    /// Creates a new [`ZendObject`] of type `T`, storing the given (and
    /// potentially uninitialized) `val` inside the object.
    ///
    /// # Parameters
    ///
    /// * `val` - Value to store inside the object. See safety section.
    /// * `init` - Whether the given `val` was initialized.
    ///
    /// # Safety
    ///
    /// Providing an initialized variant of [`MaybeUninit<T>`] is safe.
    ///
    /// Providing an uninitialized variant of [`MaybeUninit<T>`] is unsafe. As
    /// the object is uninitialized, the caller must ensure the following
    /// until the internal object is initialized:
    ///
    /// * The object is never dereferenced to `T`.
    /// * The [`Clone`] implementation is never called.
    /// * The [`Debug`] implementation is never called.
    ///
    /// If any of these conditions are not met while not initialized, the
    /// corresponding function will panic. Converting the object into its
    /// inner with the [`into_raw`] function is valid, however. You can
    /// initialize the object with the [`initialize`] function.
    ///
    /// [`into_raw`]: #method.into_raw
    /// [`initialize`]: #method.initialize
    ///
    /// # Panics
    ///
    /// Panics if memory was unable to be allocated for the new object.
    unsafe fn internal_new(val: Option<T>, ce: Option<&'static ClassEntry>) -> ZBox<Self> {
        let size = mem::size_of::<ZendClassObject<T>>();
        let meta = T::get_metadata();
        let ce = ptr::from_ref(ce.unwrap_or_else(|| meta.ce())).cast_mut();
        let obj =
            unsafe { ext_php_rs_zend_object_alloc(size as _, ce).cast::<ZendClassObject<T>>() };
        let obj = unsafe {
            obj.as_mut()
                .expect("Failed to allocate for new Zend object")
        };

        unsafe { zend_object_std_init(&raw mut obj.std, ce) };
        unsafe { object_properties_init(&raw mut obj.std, ce) };

        // SAFETY: `obj` is non-null and well aligned as it is a reference.
        // As the data in `obj.obj` is uninitialized, we don't want to drop
        // the data, but directly override it.
        unsafe { ptr::write(&raw mut obj.obj, val) };

        obj.std.handlers = meta.handlers();
        unsafe { ZBox::from_raw(obj) }
    }

    /// Initializes the class object with the value `val`.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to initialize the object with.
    ///
    /// # Returns
    ///
    /// Returns the old value in an [`Option`] if the object had already been
    /// initialized, [`None`] otherwise.
    pub fn initialize(&mut self, val: T) -> Option<T> {
        self.obj.replace(val)
    }

    /// Returns a mutable reference to the [`ZendClassObject`] of a given zend
    /// object `obj`. Returns [`None`] if the given object is not of the
    /// type `T`.
    ///
    /// # Parameters
    ///
    /// * `obj` - The zend object to get the [`ZendClassObject`] for.
    ///
    /// # Panics
    ///
    /// * If the std offset over/underflows `isize`.
    #[must_use]
    pub fn from_zend_obj(std: &zend_object) -> Option<&Self> {
        Some(Self::internal_from_zend_obj(std, true)?)
    }

    /// Returns a mutable reference to the [`ZendClassObject`] of a given zend
    /// object `obj`. Returns [`None`] if the given object is not of the
    /// type `T`.
    ///
    /// # Parameters
    ///
    /// * `obj` - The zend object to get the [`ZendClassObject`] for.
    ///
    /// # Panics
    ///
    /// * If the std offset over/underflows `isize`.
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub fn from_zend_obj_mut(std: &mut zend_object) -> Option<&mut Self> {
        Self::internal_from_zend_obj(std, true)
    }

    /// Returns a mutable reference to the [`ZendClassObject`] of a given zend
    /// object `obj`, even if the Rust object is not yet initialized.
    ///
    /// This is used internally by the constructor to get access to the object
    /// before calling `initialize()`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the returned object is not dereferenced
    /// to `T` until after `initialize()` is called. Only `initialize()` should
    /// be called on the returned object.
    ///
    /// # Parameters
    ///
    /// * `obj` - The zend object to get the [`ZendClassObject`] for.
    ///
    /// # Panics
    ///
    /// * If the std offset over/underflows `isize`.
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub(crate) fn from_zend_obj_mut_uninit(std: &mut zend_object) -> Option<&mut Self> {
        Self::internal_from_zend_obj(std, false)
    }

    // TODO: Verify if this is safe to use, as it allows mutating the
    // hashtable while only having a reference to it. #461
    #[allow(clippy::mut_from_ref)]
    fn internal_from_zend_obj(std: &zend_object, require_initialized: bool) -> Option<&mut Self> {
        // First, check if this object was created by our create_object handler.
        // We do this by comparing the handlers pointer. Objects created by PHP's
        // mock frameworks or subclasses won't have our custom handlers, and their
        // memory layout won't match ZendClassObject<T>.
        let expected_handlers = T::get_metadata().handlers();
        if !ptr::eq(std.handlers, expected_handlers) {
            return None;
        }

        let std = ptr::from_ref(std).cast::<c_char>();
        let ptr = unsafe {
            let offset = isize::try_from(Self::std_offset()).expect("Offset overflow");
            let ptr = std.offset(0 - offset).cast::<Self>();
            ptr.cast_mut().as_mut()?
        };

        if !ptr.std.instance_of(T::get_metadata().ce()) {
            return None;
        }

        // Check if the Rust object is initialized. Objects created via create_object
        // for subclasses that don't call parent::__construct() will have obj = None.
        // We must reject these to avoid panics when dereferencing.
        if require_initialized && ptr.obj.is_none() {
            return None;
        }

        Some(ptr)
    }

    /// Returns a mutable reference to the underlying Zend object.
    pub fn get_mut_zend_obj(&mut self) -> &mut zend_object {
        &mut self.std
    }

    /// Returns the offset of the `std` property in the class object.
    pub(crate) fn std_offset() -> usize {
        unsafe {
            let null = NonNull::<Self>::dangling();
            let base = null.as_ref() as *const Self;
            let std = &raw const null.as_ref().std;

            (std as usize) - (base as usize)
        }
    }
}

impl<'a, T: RegisteredClass> FromZval<'a> for &'a ZendClassObject<T> {
    const TYPE: DataType = DataType::Object(Some(T::CLASS_NAME));

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        Self::from_zend_object(zval.object()?).ok()
    }
}

impl<'a, T: RegisteredClass> FromZendObject<'a> for &'a ZendClassObject<T> {
    fn from_zend_object(obj: &'a ZendObject) -> Result<Self> {
        // TODO(david): replace with better error
        ZendClassObject::from_zend_obj(obj).ok_or(Error::InvalidScope)
    }
}

impl<'a, T: RegisteredClass> FromZvalMut<'a> for &'a mut ZendClassObject<T> {
    const TYPE: DataType = DataType::Object(Some(T::CLASS_NAME));

    fn from_zval_mut(zval: &'a mut Zval) -> Option<Self> {
        Self::from_zend_object_mut(zval.object_mut()?).ok()
    }
}

impl<'a, T: RegisteredClass> FromZendObjectMut<'a> for &'a mut ZendClassObject<T> {
    fn from_zend_object_mut(obj: &'a mut ZendObject) -> Result<Self> {
        ZendClassObject::from_zend_obj_mut(obj).ok_or(Error::InvalidScope)
    }
}

unsafe impl<T: RegisteredClass> ZBoxable for ZendClassObject<T> {
    fn free(&mut self) {
        // SAFETY: All constructors guarantee that `self` contains a valid pointer.
        // Further, all constructors guarantee that the `std` field of
        // `ZendClassObject` will be initialized.
        unsafe { ext_php_rs_zend_object_release(&raw mut self.std) }
    }
}

impl<T> Deref for ZendClassObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.obj
            .as_ref()
            .expect("Attempted to access uninitialized class object")
    }
}

impl<T> DerefMut for ZendClassObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.obj
            .as_mut()
            .expect("Attempted to access uninitialized class object")
    }
}

impl<T: RegisteredClass + Default> Default for ZBox<ZendClassObject<T>> {
    #[inline]
    fn default() -> Self {
        ZendClassObject::new(T::default())
    }
}

impl<T: RegisteredClass + Clone> Clone for ZBox<ZendClassObject<T>> {
    fn clone(&self) -> Self {
        // SAFETY: All constructors of `NewClassObject` guarantee that it will contain a
        // valid pointer. The constructor also guarantees that the internal
        // `ZendClassObject` pointer will contain a valid, initialized `obj`,
        // therefore we can dereference both safely.
        unsafe {
            let mut new = ZendClassObject::new((***self).clone());
            zend_objects_clone_members(&raw mut new.std, (&raw const self.std).cast_mut());
            new
        }
    }
}

impl<T: RegisteredClass> IntoZval for ZBox<ZendClassObject<T>> {
    const TYPE: DataType = DataType::Object(Some(T::CLASS_NAME));
    const NULLABLE: bool = false;

    fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
        let obj = self.into_raw();
        zv.set_object(&mut obj.std);
        Ok(())
    }
}

impl<T: RegisteredClass> IntoZval for &mut ZendClassObject<T> {
    const TYPE: DataType = DataType::Object(Some(T::CLASS_NAME));
    const NULLABLE: bool = false;

    #[inline]
    fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
        zv.set_object(&mut self.std);
        Ok(())
    }
}
