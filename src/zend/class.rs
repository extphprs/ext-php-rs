//! Builder and objects for creating classes in the PHP world.

use crate::ffi::instanceof_function_slow;
use crate::types::{ZendIterator, Zval};
use crate::{
    boxed::ZBox,
    convert::{FromZval, IntoZval},
    error::{Error, Result},
    ffi::{ZEND_RESULT_CODE_SUCCESS, zend_class_entry},
    flags::ClassFlags,
    types::{ZendObject, ZendStr},
    zend::ExecutorGlobals,
};
use std::ffi::CString;
use std::ptr;
use std::{convert::TryInto, fmt::Debug};

/// A PHP class entry.
///
/// Represents a class registered with the PHP interpreter.
pub type ClassEntry = zend_class_entry;

impl ClassEntry {
    /// Attempts to find a reference to a class in the global class table.
    ///
    /// Returns a reference to the class if found, or [`None`] if the class
    /// could not be found or the class table has not been initialized.
    #[must_use]
    pub fn try_find(name: &str) -> Option<&'static Self> {
        ExecutorGlobals::get().class_table()?;
        let mut name = ZendStr::new(name, false);

        unsafe {
            crate::ffi::zend_lookup_class_ex(&raw mut *name, std::ptr::null_mut(), 0).as_ref()
        }
    }

    /// Creates a new [`ZendObject`], returned inside an [`ZBox<ZendObject>`]
    /// wrapper.
    ///
    /// # Panics
    ///
    /// Panics when allocating memory for the new object fails.
    #[allow(clippy::new_ret_no_self)]
    #[must_use]
    pub fn new(&self) -> ZBox<ZendObject> {
        ZendObject::new(self)
    }

    /// Returns the class flags.
    #[must_use]
    pub fn flags(&self) -> ClassFlags {
        ClassFlags::from_bits_truncate(self.ce_flags)
    }

    /// Returns `true` if the class entry is an interface, and `false`
    /// otherwise.
    #[must_use]
    pub fn is_interface(&self) -> bool {
        self.flags().contains(ClassFlags::Interface)
    }

    /// Checks if the class is an instance of another class or interface.
    ///
    /// # Parameters
    ///
    /// * `other` - The inherited class entry to check.
    #[must_use]
    pub fn instance_of(&self, other: &ClassEntry) -> bool {
        if self == other {
            return true;
        }

        unsafe { instanceof_function_slow(ptr::from_ref(self), ptr::from_ref(other)) }
    }

    /// Returns an iterator of all the interfaces that the class implements.
    ///
    /// Returns [`None`] if the interfaces have not been resolved on the
    /// class.
    ///
    /// # Panics
    ///
    /// Panics if the number of interfaces exceeds `isize::MAX`.
    #[must_use]
    pub fn interfaces(&self) -> Option<impl Iterator<Item = &ClassEntry>> {
        self.flags()
            .contains(ClassFlags::ResolvedInterfaces)
            .then(|| unsafe {
                (0..self.num_interfaces)
                    .map(move |i| {
                        *self
                            .__bindgen_anon_3
                            .interfaces
                            .offset(isize::try_from(i).expect("index exceeds isize"))
                    })
                    .filter_map(|ptr| ptr.as_ref())
            })
    }

    /// Returns the parent of the class.
    ///
    /// If the parent of the class has not been resolved, it attempts to find
    /// the parent by name. Returns [`None`] if the parent was not resolved
    /// and the parent was not able to be found by name.
    #[must_use]
    pub fn parent(&self) -> Option<&Self> {
        if self.flags().contains(ClassFlags::ResolvedParent) {
            unsafe { self.__bindgen_anon_1.parent.as_ref() }
        } else {
            let name = unsafe { self.__bindgen_anon_1.parent_name.as_ref()? };
            Self::try_find(name.as_str().ok()?)
        }
    }

    /// Returns the iterator for the class for a specific instance
    ///
    /// Returns [`None`] if there is no associated iterator for the class.
    // TODO: Verify if this is safe to use, as it allows mutating the
    // hashtable while only having a reference to it. #461
    #[allow(clippy::mut_from_ref)]
    #[must_use]
    pub fn get_iterator<'a>(&self, zval: &'a Zval, by_ref: bool) -> Option<&'a mut ZendIterator> {
        let ptr: *const Self = self;
        let zval_ptr: *const Zval = zval;

        let iterator =
            unsafe { (*ptr).get_iterator?(ptr.cast_mut(), zval_ptr.cast_mut(), i32::from(by_ref)) };

        unsafe { iterator.as_mut() }
    }

    /// Gets the name of the class.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        unsafe { self.name.as_ref().and_then(|s| s.as_str().ok()) }
    }

    /// Reads a static property from the class.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the static property to read.
    ///
    /// # Returns
    ///
    /// Returns the value of the static property if it exists and can be
    /// converted to type `T`, or `None` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::zend::ClassEntry;
    ///
    /// let ce = ClassEntry::try_find("MyClass").unwrap();
    /// let value: Option<i64> = ce.get_static_property("counter");
    /// ```
    #[must_use]
    pub fn get_static_property<'a, T: FromZval<'a>>(&'a self, name: &str) -> Option<T> {
        let name = CString::new(name).ok()?;
        let zval = unsafe {
            crate::ffi::zend_read_static_property(
                ptr::from_ref(self).cast_mut(),
                name.as_ptr(),
                name.as_bytes().len(),
                true, // silent - don't throw if property doesn't exist
            )
            .as_ref()?
        };
        T::from_zval(zval)
    }

    /// Sets a static property on the class.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the static property to set.
    /// * `value` - The value to set the property to.
    ///
    /// # Errors
    ///
    /// Returns an error if the property name contains a null byte, if the
    /// value could not be converted to a Zval, or if the property could not
    /// be updated (e.g., the property does not exist).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::zend::ClassEntry;
    ///
    /// let ce = ClassEntry::try_find("MyClass").unwrap();
    /// ce.set_static_property("counter", 42).unwrap();
    /// ```
    pub fn set_static_property<T: IntoZval>(&self, name: &str, value: T) -> Result<()> {
        let name = CString::new(name)?;
        let mut zval = value.into_zval(false)?;
        let result = unsafe {
            crate::ffi::zend_update_static_property(
                ptr::from_ref(self).cast_mut(),
                name.as_ptr(),
                name.as_bytes().len(),
                &raw mut zval,
            )
        };
        if result == ZEND_RESULT_CODE_SUCCESS {
            Ok(())
        } else {
            Err(Error::InvalidProperty)
        }
    }
}

impl PartialEq for ClassEntry {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Debug for ClassEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name: String = unsafe { self.name.as_ref() }
            .and_then(|s| s.try_into().ok())
            .ok_or(std::fmt::Error)?;

        f.debug_struct("ClassEntry")
            .field("name", &name)
            .field("flags", &self.flags())
            .field("is_interface", &self.is_interface())
            .field(
                "interfaces",
                &self.interfaces().map(Iterator::collect::<Vec<_>>),
            )
            .field("parent", &self.parent())
            .finish()
    }
}
