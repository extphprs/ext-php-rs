//! Reports the PHP values held in the Rust fields of a class to the PHP cycle
//! collector.
//!
//! The collector finds a cycle only when every object in it tells which
//! values it holds. The [`macro@crate::php_class`] macro does this for each
//! field whose type implements [`GcTrace`].

use std::{
    collections::{BTreeMap, HashMap},
    os::raw::c_int,
    ptr,
};

use crate::{
    boxed::ZBox,
    class::RegisteredClass,
    ffi::{zend_get_gc_buffer, zend_get_gc_buffer_create, zend_get_gc_buffer_grow},
    flags::ZvalTypeFlags,
    types::{ZendClassObject, ZendObject, Zval},
};

/// The values that one object reports to the cycle collector.
///
/// Only the `get_gc` handler of a [`RegisteredClass`] creates it, and gives it
/// to [`RegisteredClass::gc_trace`]. Only [`GcTrace`] implementations can add
/// values to it.
pub struct GcRoots {
    buffer: *mut zend_get_gc_buffer,
}

impl GcRoots {
    pub(crate) fn new() -> Self {
        Self {
            buffer: ptr::null_mut(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.buffer.is_null()
    }

    /// `zend_get_gc_buffer_add_zval`: copies `value` without a new reference.
    pub(crate) fn push(&mut self, value: &Zval) {
        // SAFETY: every variant of the `u1` union is a plain integer.
        let type_info = unsafe { value.u1.type_info };
        if ZvalTypeFlags::from_bits_retain(type_info).contains(ZvalTypeFlags::RefCounted) {
            let slot = self.next_slot();
            // SAFETY: `next_slot` returns a writable slot of the engine buffer.
            unsafe {
                (*slot).value = value.value;
                (*slot).u1 = value.u1;
            }
        }
    }

    /// `zend_get_gc_buffer_add_obj`.
    pub(crate) fn push_object(&mut self, object: &ZendObject) {
        let slot = self.next_slot();
        // SAFETY: `next_slot` returns a writable slot of the engine buffer.
        unsafe {
            (*slot).value.obj = ptr::from_ref(object).cast_mut();
            (*slot).u1.type_info = ZvalTypeFlags::ObjectEx.bits();
        }
    }

    fn next_slot(&mut self) -> *mut Zval {
        // SAFETY: the collector calls `get_gc` with the engine running, and
        // only one `get_gc` call is active at a time, so the buffer is ours.
        unsafe {
            if self.buffer.is_null() {
                self.buffer = zend_get_gc_buffer_create();
            }
            let buffer = &mut *self.buffer;
            if buffer.cur == buffer.end {
                zend_get_gc_buffer_grow(buffer);
            }
            let slot = buffer.cur;
            buffer.cur = buffer.cur.add(1);
            slot
        }
    }

    /// `zend_get_gc_buffer_use`.
    ///
    /// # Safety
    ///
    /// The roots must not be empty, and `table` and `n` must be the out
    /// pointers of a `get_gc` call.
    pub(crate) unsafe fn write_table(self, table: *mut *mut Zval, n: *mut c_int) {
        // SAFETY: the caller guarantees a pushed value, so `buffer` is the
        // engine buffer and `cur` is in the same allocation as `start`.
        unsafe {
            let buffer = &*self.buffer;
            *table = buffer.start;
            *n = c_int::try_from(buffer.cur.offset_from(buffer.start)).unwrap_or(c_int::MAX);
        }
    }
}

/// A value that holds references to PHP values.
///
/// [`macro@crate::php_class`] calls [`GcTrace::gc_trace`] for each field of
/// the class whose type implements this trait. The PHP cycle collector can
/// then free a cycle that goes through the field, for example an object that
/// keeps a closure which captures the same object.
///
/// ext-php-rs implements it for [`Zval`], [`ZBox<ZendObject>`],
/// [`ZBox<ZendClassObject<T>>`], an owned [`ZendCallable`] and the standard
/// containers of these types. It does not implement it for
/// `ZBox<ZendHashTable>`: keep an array in a [`Zval`] to make it visible.
///
/// # Safety
///
/// An implementation must report only the PHP references that `self` owns,
/// each one time, and report the same values while `self` does not change. It
/// must not call PHP code and must not panic. If a reference is reported two
/// times, or is not owned, the collector can free a value that is still in
/// use.
///
/// [`ZendCallable`]: crate::types::ZendCallable
pub unsafe trait GcTrace {
    /// Adds the PHP references owned by `self` to `roots`.
    ///
    /// # Safety
    ///
    /// Call it at most one time for each value during one `get_gc` call.
    unsafe fn gc_trace(&self, roots: &mut GcRoots);
}

unsafe impl GcTrace for Zval {
    unsafe fn gc_trace(&self, roots: &mut GcRoots) {
        roots.push(self);
    }
}

unsafe impl GcTrace for ZBox<ZendObject> {
    unsafe fn gc_trace(&self, roots: &mut GcRoots) {
        if !self.is_released() {
            roots.push_object(self);
        }
    }
}

unsafe impl<T: RegisteredClass> GcTrace for ZBox<ZendClassObject<T>> {
    unsafe fn gc_trace(&self, roots: &mut GcRoots) {
        if !self.is_released() {
            roots.push_object(&self.std);
        }
    }
}

unsafe impl<T: GcTrace> GcTrace for Option<T> {
    unsafe fn gc_trace(&self, roots: &mut GcRoots) {
        if let Some(value) = self {
            // SAFETY: `self` owns its value, which nothing else reports.
            unsafe { value.gc_trace(roots) };
        }
    }
}

unsafe impl<T: GcTrace + ?Sized> GcTrace for Box<T> {
    unsafe fn gc_trace(&self, roots: &mut GcRoots) {
        // SAFETY: the box owns its value, which nothing else reports.
        unsafe { (**self).gc_trace(roots) };
    }
}

unsafe impl<T: GcTrace> GcTrace for [T] {
    unsafe fn gc_trace(&self, roots: &mut GcRoots) {
        for value in self {
            // SAFETY: each element is a distinct value, reported one time.
            unsafe { value.gc_trace(roots) };
        }
    }
}

unsafe impl<T: GcTrace, const N: usize> GcTrace for [T; N] {
    unsafe fn gc_trace(&self, roots: &mut GcRoots) {
        // SAFETY: the slice reports each element one time.
        unsafe { self.as_slice().gc_trace(roots) };
    }
}

unsafe impl<T: GcTrace> GcTrace for Vec<T> {
    unsafe fn gc_trace(&self, roots: &mut GcRoots) {
        // SAFETY: the slice reports each element one time.
        unsafe { self.as_slice().gc_trace(roots) };
    }
}

unsafe impl<K, V: GcTrace, S> GcTrace for HashMap<K, V, S> {
    unsafe fn gc_trace(&self, roots: &mut GcRoots) {
        for value in self.values() {
            // SAFETY: each value is a distinct entry, reported one time.
            unsafe { value.gc_trace(roots) };
        }
    }
}

unsafe impl<K, V: GcTrace> GcTrace for BTreeMap<K, V> {
    unsafe fn gc_trace(&self, roots: &mut GcRoots) {
        for value in self.values() {
            // SAFETY: each value is a distinct entry, reported one time.
            unsafe { value.gc_trace(roots) };
        }
    }
}
