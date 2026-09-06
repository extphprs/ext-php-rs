//! A pointer type for heap allocation using the Zend memory manager.
//!
//! Heap memory in PHP is usually request-bound and allocated inside [memory
//! arenas], which are cleared at the start and end of a PHP request. Allocating
//! and freeing memory that is allocated on the Zend heap is done through two
//! separate functions [`efree`] and [`emalloc`].
//!
//! As such, most heap-allocated PHP types **cannot** be allocated on the stack,
//! such as [`ZendStr`], which is a dynamically-sized type, and therefore must
//! be allocated on the heap. A regular [`Box`] would not work in this case, as
//! the memory needs to be freed from a separate function `zend_string_release`.
//! The [`ZBox`] type provides a wrapper which calls the relevant release
//! functions based on the type and what is inside the implementation of
//! [`ZBoxable`].
//!
//! This type is not created directly, but rather through a function implemented
//! on the downstream type. For example, [`ZendStr`] has a function `new` which
//! returns a [`ZBox<ZendStr>`].
//!
//! [memory arenas]: https://en.wikipedia.org/wiki/Region-based_memory_management
//! [`ZendStr`]: crate::types::ZendStr
//! [`emalloc`]: super::alloc::emalloc

use std::{
    borrow::Borrow,
    fmt::Debug,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
};

use super::alloc::efree;

/// A pointer type for heap allocation using the Zend memory manager.
///
/// See the [module level documentation](../index.html) for more.
pub struct ZBox<T: ZBoxable>(NonNull<T>);

impl<T: ZBoxable> ZBox<T> {
    /// Creates a new box from a given pointer.
    ///
    /// # Parameters
    ///
    /// * `ptr` - A non-null, well-aligned pointer to a `T`.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `ptr` is non-null, well-aligned and pointing to
    /// a `T`.
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        Self(unsafe { NonNull::new_unchecked(ptr) })
    }

    /// Returns the pointer contained by the box, dropping the box in the
    /// process. The data pointed to by the returned pointer is not
    /// released.
    ///
    /// Mirrors [`Box::into_raw`]: the pointer is non-null and well-aligned, and
    /// the caller becomes responsible for releasing it, typically by passing it
    /// back to [`ZBox::from_raw`] or to the engine.
    #[must_use]
    pub fn into_raw(self) -> *mut T {
        ManuallyDrop::new(self).0.as_ptr()
    }
}

impl<T: ZBoxable> Drop for ZBox<T> {
    #[inline]
    fn drop(&mut self) {
        self.deref_mut().free();
    }
}

impl<T: ZBoxable> Deref for ZBox<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: All constructors ensure the contained pointer is well-aligned and
        // dereferenceable.
        unsafe { self.0.as_ref() }
    }
}

impl<T: ZBoxable> DerefMut for ZBox<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: All constructors ensure the contained pointer is well-aligned and
        // dereferenceable.
        unsafe { self.0.as_mut() }
    }
}

impl<T: ZBoxable + Debug> Debug for ZBox<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ZBoxable> Borrow<T> for ZBox<T> {
    #[inline]
    fn borrow(&self) -> &T {
        self
    }
}

impl<T: ZBoxable> AsRef<T> for ZBox<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self
    }
}

/// Implemented on types that can be heap allocated using the Zend memory
/// manager. These types are stored inside a [`ZBox`] when heap-allocated, and
/// the [`free`] method is called when the box is dropped.
///
/// # Safety
///
/// The default implementation of the [`free`] function uses the [`efree`]
/// function to free the memory without calling any destructors.
///
/// The implementor must ensure that any time a pointer to the implementor is
/// passed into a [`ZBox`] that the memory pointed to was allocated by the Zend
/// memory manager.
///
/// [`free`]: #method.free
pub unsafe trait ZBoxable {
    /// Frees the memory pointed to by `self`, calling any destructors required
    /// in the process.
    fn free(&mut self) {
        unsafe { efree(ptr::from_mut(self).cast::<u8>()) };
    }
}
