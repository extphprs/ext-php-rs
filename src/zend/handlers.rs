use std::{ffi::CString, ffi::c_void, mem::MaybeUninit, os::raw::c_int, ptr};

use crate::{
    class::RegisteredClass,
    exception::PhpResult,
    ffi::{
        ext_php_rs_executor_globals, instanceof_function_slow, std_object_handlers,
        zend_class_entry, zend_is_true, zend_object_handlers, zend_object_std_dtor,
        zend_std_get_properties, zend_std_has_property, zend_std_read_property,
        zend_std_write_property, zend_throw_error,
    },
    flags::{PropertyFlags, ZvalTypeFlags},
    types::{ZendClassObject, ZendHashTable, ZendObject, ZendStr, Zval},
};

/// A set of functions associated with a PHP class.
pub type ZendObjectHandlers = zend_object_handlers;

impl ZendObjectHandlers {
    /// Creates a new set of object handlers based on the standard object
    /// handlers.
    #[must_use]
    pub fn new<T: RegisteredClass>() -> ZendObjectHandlers {
        let mut this = MaybeUninit::uninit();

        // SAFETY: `this` is allocated on the stack and is a valid memory location.
        unsafe { Self::init::<T>(&raw mut *this.as_mut_ptr()) };

        // SAFETY: We just initialized the handlers in the previous statement, therefore
        // we are returning a valid object.
        unsafe { this.assume_init() }
    }

    /// Initializes a given set of object handlers by copying the standard
    /// object handlers into the memory location, as well as setting up the
    /// `T` type destructor.
    ///
    /// # Parameters
    ///
    /// * `ptr` - Pointer to memory location to copy the standard handlers to.
    ///
    /// # Safety
    ///
    /// Caller must guarantee that the `ptr` given is a valid memory location.
    ///
    /// # Panics
    ///
    /// * If the offset of the `T` type is not a valid `i32` value.
    pub unsafe fn init<T: RegisteredClass>(ptr: *mut ZendObjectHandlers) {
        unsafe { ptr::copy_nonoverlapping(&raw const std_object_handlers, ptr, 1) };
        let offset = ZendClassObject::<T>::std_offset();
        unsafe { (*ptr).offset = offset.try_into().expect("Invalid offset") };
        unsafe { (*ptr).free_obj = Some(Self::free_obj::<T>) };
        unsafe { (*ptr).read_property = Some(Self::read_property::<T>) };
        unsafe { (*ptr).write_property = Some(Self::write_property::<T>) };
        unsafe { (*ptr).get_properties = Some(Self::get_properties::<T>) };
        unsafe { (*ptr).has_property = Some(Self::has_property::<T>) };
    }

    unsafe extern "C" fn free_obj<T: RegisteredClass>(object: *mut ZendObject) {
        let obj = unsafe {
            object
                .as_mut()
                .and_then(|obj| ZendClassObject::<T>::from_zend_obj_mut(obj))
                .expect("Invalid object pointer given for `free_obj`")
        };

        // Manually drop the object as we don't want to free the underlying memory.
        unsafe { ptr::drop_in_place(&raw mut obj.obj) };

        unsafe { zend_object_std_dtor(object) };
    }

    unsafe extern "C" fn read_property<T: RegisteredClass>(
        object: *mut ZendObject,
        member: *mut ZendStr,
        type_: c_int,
        cache_slot: *mut *mut c_void,
        rv: *mut Zval,
    ) -> *mut Zval {
        // TODO: Measure this
        #[allow(clippy::inline_always)]
        #[inline(always)]
        unsafe fn internal<T: RegisteredClass>(
            object: *mut ZendObject,
            member: *mut ZendStr,
            type_: c_int,
            cache_slot: *mut *mut c_void,
            rv: *mut Zval,
        ) -> PhpResult<*mut Zval> {
            let obj = unsafe {
                object
                    .as_mut()
                    .and_then(|obj| ZendClassObject::<T>::from_zend_obj_mut(obj))
                    .ok_or("Invalid object pointer given")?
            };
            let prop_name = unsafe {
                member
                    .as_ref()
                    .ok_or("Invalid property name pointer given")?
            };
            let self_ = &mut *obj;
            let props = T::get_metadata().get_properties();
            let prop = props.get(prop_name.as_str()?);

            // retval needs to be treated as initialized, so we set the type to null
            let rv_mut = unsafe { rv.as_mut().ok_or("Invalid return zval given")? };
            rv_mut.u1.type_info = ZvalTypeFlags::Null.bits();

            Ok(match prop {
                Some(prop_info) => {
                    // Check visibility before allowing access
                    let object_ce = unsafe { (*object).ce };
                    if !unsafe { check_property_access(prop_info.flags, object_ce) } {
                        let is_private = prop_info.flags.contains(PropertyFlags::Private);
                        unsafe {
                            throw_property_access_error(
                                T::CLASS_NAME,
                                prop_name.as_str()?,
                                is_private,
                            );
                        }
                        return Ok(rv);
                    }
                    prop_info.prop.get(self_, rv_mut)?;
                    rv
                }
                None => unsafe { zend_std_read_property(object, member, type_, cache_slot, rv) },
            })
        }

        match unsafe { internal::<T>(object, member, type_, cache_slot, rv) } {
            Ok(rv) => rv,
            Err(e) => {
                let _ = e.throw();
                unsafe { (*rv).set_null() };
                rv
            }
        }
    }

    unsafe extern "C" fn write_property<T: RegisteredClass>(
        object: *mut ZendObject,
        member: *mut ZendStr,
        value: *mut Zval,
        cache_slot: *mut *mut c_void,
    ) -> *mut Zval {
        // TODO: Measure this
        #[allow(clippy::inline_always)]
        #[inline(always)]
        unsafe fn internal<T: RegisteredClass>(
            object: *mut ZendObject,
            member: *mut ZendStr,
            value: *mut Zval,
            cache_slot: *mut *mut c_void,
        ) -> PhpResult<*mut Zval> {
            let obj = unsafe {
                object
                    .as_mut()
                    .and_then(|obj| ZendClassObject::<T>::from_zend_obj_mut(obj))
                    .ok_or("Invalid object pointer given")?
            };
            let prop_name = unsafe {
                member
                    .as_ref()
                    .ok_or("Invalid property name pointer given")?
            };
            let self_ = &mut *obj;
            let props = T::get_metadata().get_properties();
            let prop = props.get(prop_name.as_str()?);
            let value_mut = unsafe { value.as_mut().ok_or("Invalid return zval given")? };

            Ok(match prop {
                Some(prop_info) => {
                    // Check visibility before allowing access
                    let object_ce = unsafe { (*object).ce };
                    if !unsafe { check_property_access(prop_info.flags, object_ce) } {
                        let is_private = prop_info.flags.contains(PropertyFlags::Private);
                        unsafe {
                            throw_property_access_error(
                                T::CLASS_NAME,
                                prop_name.as_str()?,
                                is_private,
                            );
                        }
                        return Ok(value);
                    }
                    prop_info.prop.set(self_, value_mut)?;
                    value
                }
                None => unsafe { zend_std_write_property(object, member, value, cache_slot) },
            })
        }

        match unsafe { internal::<T>(object, member, value, cache_slot) } {
            Ok(rv) => rv,
            Err(e) => {
                let _ = e.throw();
                value
            }
        }
    }

    unsafe extern "C" fn get_properties<T: RegisteredClass>(
        object: *mut ZendObject,
    ) -> *mut ZendHashTable {
        // TODO: Measure this
        #[allow(clippy::inline_always)]
        #[inline(always)]
        unsafe fn internal<T: RegisteredClass>(
            object: *mut ZendObject,
            props: &mut ZendHashTable,
        ) -> PhpResult {
            let obj = unsafe {
                object
                    .as_mut()
                    .and_then(|obj| ZendClassObject::<T>::from_zend_obj_mut(obj))
                    .ok_or("Invalid object pointer given")?
            };
            let self_ = &mut *obj;
            let struct_props = T::get_metadata().get_properties();

            for (&name, val) in struct_props {
                let mut zv = Zval::new();
                if val.prop.get(self_, &mut zv).is_err() {
                    continue;
                }

                // Mangle property name according to visibility for debug output
                // PHP convention: private = "\0ClassName\0propName", protected =
                // "\0*\0propName"
                let mangled_name = if val.flags.contains(PropertyFlags::Private) {
                    format!("\0{}\0{name}", T::CLASS_NAME)
                } else if val.flags.contains(PropertyFlags::Protected) {
                    format!("\0*\0{name}")
                } else {
                    name.to_string()
                };

                props.insert(mangled_name.as_str(), zv).map_err(|e| {
                    format!("Failed to insert value into properties hashtable: {e:?}")
                })?;
            }

            Ok(())
        }

        let props = unsafe {
            zend_std_get_properties(object)
                .as_mut()
                .or_else(|| Some(ZendHashTable::new().into_raw()))
                .expect("Failed to get property hashtable")
        };

        if let Err(e) = unsafe { internal::<T>(object, props) } {
            let _ = e.throw();
        }

        props
    }

    unsafe extern "C" fn has_property<T: RegisteredClass>(
        object: *mut ZendObject,
        member: *mut ZendStr,
        has_set_exists: c_int,
        cache_slot: *mut *mut c_void,
    ) -> c_int {
        // TODO: Measure this
        #[allow(clippy::inline_always)]
        #[inline(always)]
        unsafe fn internal<T: RegisteredClass>(
            object: *mut ZendObject,
            member: *mut ZendStr,
            has_set_exists: c_int,
            cache_slot: *mut *mut c_void,
        ) -> PhpResult<c_int> {
            let obj = unsafe {
                object
                    .as_mut()
                    .and_then(|obj| ZendClassObject::<T>::from_zend_obj_mut(obj))
                    .ok_or("Invalid object pointer given")?
            };
            let prop_name = unsafe {
                member
                    .as_ref()
                    .ok_or("Invalid property name pointer given")?
            };
            let props = T::get_metadata().get_properties();
            let prop = props.get(prop_name.as_str()?);
            let self_ = &mut *obj;

            match has_set_exists {
                //
                // * 0 (has) whether property exists and is not NULL
                0 => {
                    if let Some(val) = prop {
                        let mut zv = Zval::new();
                        val.prop.get(self_, &mut zv)?;
                        if !zv.is_null() {
                            return Ok(1);
                        }
                    }
                }
                //
                // * 1 (set) whether property exists and is true
                1 => {
                    if let Some(val) = prop {
                        let mut zv = Zval::new();
                        val.prop.get(self_, &mut zv)?;

                        cfg_if::cfg_if! {
                            if #[cfg(php84)] {
                                #[allow(clippy::unnecessary_mut_passed)]
                                if unsafe { zend_is_true(&raw mut zv) } {
                                    return Ok(1);
                                }
                            } else {
                                #[allow(clippy::unnecessary_mut_passed)]
                                if unsafe { zend_is_true(&raw mut zv) } == 1 {
                                    return Ok(1);
                                }
                            }
                        }
                    }
                }
                //
                // * 2 (exists) whether property exists
                2 => {
                    if prop.is_some() {
                        return Ok(1);
                    }
                }
                _ => return Err(
                    "Invalid value given for `has_set_exists` in struct `has_property` function."
                        .into(),
                ),
            }

            Ok(unsafe { zend_std_has_property(object, member, has_set_exists, cache_slot) })
        }

        match unsafe { internal::<T>(object, member, has_set_exists, cache_slot) } {
            Ok(rv) => rv,
            Err(e) => {
                let _ = e.throw();
                0
            }
        }
    }
}

/// Gets the current calling scope from the executor globals.
///
/// # Safety
///
/// Must only be called during PHP execution when executor globals are valid.
#[inline]
unsafe fn get_calling_scope() -> *const zend_class_entry {
    let eg = unsafe { ext_php_rs_executor_globals().as_ref() };
    let Some(eg) = eg else {
        return ptr::null();
    };
    let execute_data = eg.current_execute_data;

    if execute_data.is_null() {
        return ptr::null();
    }

    let func = unsafe { (*execute_data).func };
    if func.is_null() {
        return ptr::null();
    }

    // Access the common.scope field through the union
    unsafe { (*func).common.scope }
}

/// Checks if the calling scope has access to a property with the given flags.
///
/// Returns `true` if access is allowed, `false` otherwise.
///
/// # Safety
///
/// Must only be called during PHP execution when executor globals are valid.
/// The `object_ce` pointer must be valid.
#[inline]
unsafe fn check_property_access(flags: PropertyFlags, object_ce: *const zend_class_entry) -> bool {
    // Public properties are always accessible
    if !flags.contains(PropertyFlags::Private) && !flags.contains(PropertyFlags::Protected) {
        return true;
    }

    let calling_scope = unsafe { get_calling_scope() };

    if flags.contains(PropertyFlags::Private) {
        // Private: must be called from the exact same class
        return calling_scope == object_ce;
    }

    if flags.contains(PropertyFlags::Protected) {
        // Protected: must be called from same class or a subclass
        if calling_scope.is_null() {
            return false;
        }

        // Same class check
        if calling_scope == object_ce {
            return true;
        }

        // Check if calling_scope is a subclass of object_ce
        // or if object_ce is a subclass of calling_scope (for parent access)
        unsafe {
            instanceof_function_slow(calling_scope, object_ce)
                || instanceof_function_slow(object_ce, calling_scope)
        }
    } else {
        true
    }
}

/// Throws an error for invalid property access.
///
/// # Safety
///
/// Must only be called during PHP execution.
///
/// # Panics
///
/// Panics if the error message cannot be converted to a `CString`.
unsafe fn throw_property_access_error(class_name: &str, prop_name: &str, is_private: bool) {
    let visibility = if is_private { "private" } else { "protected" };
    let message = CString::new(format!(
        "Cannot access {visibility} property {class_name}::${prop_name}"
    ))
    .expect("Failed to create error message");

    unsafe {
        zend_throw_error(ptr::null_mut(), message.as_ptr());
    }
}
