//! Represents an object in PHP. Allows for overriding the internal object used
//! by classes, allowing users to store Rust data inside a PHP object.

use std::{
    cell::RefCell,
    fmt::Debug,
    mem,
    os::raw::c_char,
    ptr::{self, NonNull},
};

use crate::{
    boxed::{ZBox, ZBoxable},
    class::RegisteredClass,
    convert::{FromZendObject, FromZval, IntoZval},
    error::{Error, Result},
    ffi::{
        ext_php_rs_zend_object_alloc, ext_php_rs_zend_object_release, object_properties_init,
        zend_object, zend_object_std_init, zend_objects_clone_members,
    },
    flags::DataType,
    types::{
        ZendObject, Zval,
        payload::{self, BorrowError, ClassRef, ClassRefMut},
    },
    zend::ClassEntry,
};

/// Representation of a Zend class object in memory.
#[repr(C)]
#[derive(Debug)]
pub struct ZendClassObject<T> {
    obj: RefCell<Option<T>>,
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
        Self::internal_new(Some(val), None)
    }

    /// Creates a new [`ZendClassObject`] of type `T`, with an uninitialized
    /// internal object.
    ///
    /// Until [`initialize`] is called, [`try_borrow`] and [`try_borrow_mut`]
    /// return [`BorrowError::Uninitialized`] and the [`Clone`]
    /// implementation panics.
    ///
    /// [`initialize`]: #method.initialize
    /// [`try_borrow`]: #method.try_borrow
    /// [`try_borrow_mut`]: #method.try_borrow_mut
    ///
    /// # Panics
    ///
    /// Panics if memory was unable to be allocated for the new object.
    pub fn new_uninit(ce: Option<&'static ClassEntry>) -> ZBox<Self> {
        Self::internal_new(None, ce)
    }

    #[expect(
        clippy::expect_used,
        reason = "the Zend allocator aborts the process on OOM, so the pointer is never null"
    )]
    fn internal_new(val: Option<T>, ce: Option<&'static ClassEntry>) -> ZBox<Self> {
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
        unsafe { ptr::write(&raw mut obj.obj, RefCell::new(val)) };

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
    ///
    /// # Errors
    ///
    /// * [`BorrowError::Borrowed`] - The value is borrowed.
    pub fn initialize(&self, val: T) -> Result<Option<T>, BorrowError> {
        payload::replace(&self.obj, val, T::CLASS_NAME)
    }

    /// Borrows the Rust value of the object.
    ///
    /// # Errors
    ///
    /// * [`BorrowError::MutablyBorrowed`] - The value is borrowed mutably.
    /// * [`BorrowError::Uninitialized`] - The object has no Rust value.
    pub fn try_borrow(&self) -> Result<ClassRef<'_, T>, BorrowError> {
        payload::borrow(&self.obj, T::CLASS_NAME)
    }

    /// Borrows the Rust value of the object mutably.
    ///
    /// # Errors
    ///
    /// * [`BorrowError::Borrowed`] - The value is borrowed.
    /// * [`BorrowError::Uninitialized`] - The object has no Rust value.
    pub fn try_borrow_mut(&self) -> Result<ClassRefMut<'_, T>, BorrowError> {
        payload::borrow_mut(&self.obj, T::CLASS_NAME)
    }

    /// Returns the Rust value without a borrow guard, or [`None`] while a
    /// [`ClassRefMut`] lives or the object is not initialized.
    ///
    /// # Safety
    ///
    /// The value must not be borrowed mutably while the returned reference
    /// lives.
    pub(crate) unsafe fn unguarded(&self) -> Option<&T> {
        // SAFETY: the caller keeps the value free of mutable borrows while the
        // reference lives.
        unsafe { self.obj.try_borrow_unguarded() }
            .ok()
            .and_then(Option::as_ref)
    }

    /// Moves the Rust value out of the object, ignoring the borrow flag.
    ///
    /// # Safety
    ///
    /// No borrow of the value may be used again. The engine calls `free_obj`
    /// once the refcount is zero, so the only guards left are the ones a
    /// bailout skipped.
    pub(crate) unsafe fn take_unguarded(&self) -> Option<T> {
        // SAFETY: the caller makes sure that no borrow of the value is used
        // again, so the write through the cell pointer aliases nothing.
        unsafe { ptr::replace(self.obj.as_ptr(), None) }
    }

    /// Returns a reference to the [`ZendClassObject`] of a given zend
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
        // SAFETY: `std` is a live `zend_object` for the lifetime of the borrow,
        // and the resolved pointer is only ever read through a shared reference.
        unsafe { Self::resolve(ptr::from_ref(std), true)?.as_ref() }
    }

    /// Returns a reference to the [`ZendClassObject`] of a given zend object
    /// `obj`, even if the Rust object is not yet initialized. Returns
    /// [`None`] if the given object is not of the type `T`.
    ///
    /// # Parameters
    ///
    /// * `obj` - The zend object to get the [`ZendClassObject`] for.
    ///
    /// # Panics
    ///
    /// * If the std offset over/underflows `isize`.
    pub(crate) fn from_zend_obj_uninit(std: &zend_object) -> Option<&Self> {
        // SAFETY: `std` is a live `zend_object` for the lifetime of the borrow,
        // and the resolved pointer is only ever read through a shared reference.
        unsafe { Self::resolve(ptr::from_ref(std), false)?.as_ref() }
    }

    /// Resolves the [`ZendClassObject`] containing `std`, validating that the
    /// object was created by this type's `create_object` handler.
    ///
    /// # Safety
    ///
    /// `std` must be non-null and point to a live `zend_object` for the duration
    /// of the call.
    ///
    /// # Panics
    ///
    /// * If the std offset over/underflows `isize`.
    #[expect(
        clippy::expect_used,
        reason = "the offset is a compile-time struct offset and fits in isize"
    )]
    unsafe fn resolve(std: *const zend_object, require_initialized: bool) -> Option<*const Self> {
        // First, check if this object was created by our create_object handler.
        // We do this by comparing the handlers pointer. Objects created by PHP's
        // mock frameworks or subclasses won't have our custom handlers, and their
        // memory layout won't match ZendClassObject<T>.
        let expected_handlers = T::get_metadata().handlers();
        if !ptr::eq(unsafe { (*std).handlers }, expected_handlers) {
            return None;
        }

        let this = unsafe {
            let offset = isize::try_from(Self::std_offset()).expect("Offset overflow");
            std.cast::<c_char>().offset(0 - offset).cast::<Self>()
        };
        let this_ref = unsafe { this.as_ref()? };

        if !this_ref.std.instance_of(T::get_metadata().ce()) {
            return None;
        }

        // Check if the Rust object is initialized. Objects created via create_object
        // for subclasses that don't call parent::__construct() will have obj = None.
        // We must reject these to avoid panics when dereferencing.
        if require_initialized && !this_ref.is_initialized() {
            return None;
        }

        Some(this)
    }

    fn is_initialized(&self) -> bool {
        // SAFETY: the reference is dropped before this function returns. A
        // `ClassRefMut` only exists over an initialized value.
        unsafe { self.obj.try_borrow_unguarded() }.map_or(true, Option::is_some)
    }

    /// Returns a mutable reference to the underlying Zend object.
    pub fn get_mut_zend_obj(&mut self) -> &mut zend_object {
        &mut self.std
    }

    /// Returns the offset of the `std` property in the class object.
    pub(crate) fn std_offset() -> usize {
        unsafe {
            let null = NonNull::<Self>::dangling();
            let base = ptr::from_ref::<Self>(null.as_ref());
            let std = &raw const null.as_ref().std;

            (std as usize) - (base as usize)
        }
    }
}

impl<'a, T: RegisteredClass> FromZval<'a> for &'a ZendClassObject<T> {
    const TYPE: DataType = DataType::object(T::CLASS_NAME);

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

impl<'a, T: RegisteredClass> FromZval<'a> for ClassRef<'a, T> {
    const TYPE: DataType = DataType::object(T::CLASS_NAME);

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        Self::from_zend_object(zval.object()?).ok()
    }
}

impl<'a, T: RegisteredClass> FromZendObject<'a> for ClassRef<'a, T> {
    fn from_zend_object(obj: &'a ZendObject) -> Result<Self> {
        let obj = ZendClassObject::<T>::from_zend_obj(obj).ok_or(Error::InvalidScope)?;
        Ok(obj.try_borrow()?)
    }
}

impl<'a, T: RegisteredClass> FromZval<'a> for ClassRefMut<'a, T> {
    const TYPE: DataType = DataType::object(T::CLASS_NAME);

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        Self::from_zend_object(zval.object()?).ok()
    }
}

impl<'a, T: RegisteredClass> FromZendObject<'a> for ClassRefMut<'a, T> {
    fn from_zend_object(obj: &'a ZendObject) -> Result<Self> {
        let obj = ZendClassObject::<T>::from_zend_obj(obj).ok_or(Error::InvalidScope)?;
        Ok(obj.try_borrow_mut()?)
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

impl<T: RegisteredClass + Default> Default for ZBox<ZendClassObject<T>> {
    #[inline]
    fn default() -> Self {
        ZendClassObject::new(T::default())
    }
}

impl<T: RegisteredClass + Clone> Clone for ZBox<ZendClassObject<T>> {
    /// # Panics
    ///
    /// Panics with the [`BorrowError`] message if the value is borrowed
    /// mutably or not initialized.
    fn clone(&self) -> Self {
        let value = match self.try_borrow() {
            Ok(value) => T::clone(&value),
            Err(err) => panic!("{err}"),
        };
        let mut new = ZendClassObject::new(value);
        // SAFETY: both objects are live, and the source guard is released.
        unsafe { zend_objects_clone_members(&raw mut new.std, (&raw const self.std).cast_mut()) };
        new
    }
}

impl<T: RegisteredClass> IntoZval for ZBox<ZendClassObject<T>> {
    const TYPE: DataType = DataType::object(T::CLASS_NAME);
    const NULLABLE: bool = false;

    fn set_zval(mut self, zv: &mut Zval, _: bool) -> Result<()> {
        // `set_object` adds a reference on insertion, so del_ref first to keep the
        // net refcount at 1. Matches `ZBox<ZendObject>::set_zval` in object.rs.
        self.std.del_ref();
        let obj = self.into_raw();
        // SAFETY: `into_raw` yields a valid, exclusively owned object whose
        // reference is transferred to the zval.
        zv.set_object(unsafe { &mut (*obj).std });
        Ok(())
    }
}

impl<T: RegisteredClass> IntoZval for &ZendClassObject<T> {
    const TYPE: DataType = DataType::object(T::CLASS_NAME);
    const NULLABLE: bool = false;

    #[inline]
    fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
        // SAFETY: `self.std` is a live object; the zval takes a new reference.
        unsafe { zv.set_object_ptr((&raw const self.std).cast_mut()) };
        Ok(())
    }
}
