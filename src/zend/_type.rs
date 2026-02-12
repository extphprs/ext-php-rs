use std::{ffi::c_void, ptr};

use crate::{
    ffi::{
        _IS_BOOL, _ZEND_IS_VARIADIC_BIT, _ZEND_SEND_MODE_SHIFT, _ZEND_TYPE_INTERSECTION_BIT,
        _ZEND_TYPE_LIST_BIT, _ZEND_TYPE_NULLABLE_BIT, _ZEND_TYPE_UNION_BIT, IS_MIXED, MAY_BE_ANY,
        MAY_BE_BOOL, ext_php_rs_pemalloc, zend_type, zend_type_list,
    },
    flags::DataType,
};

/// Internal Zend type.
pub type ZendType = zend_type;

impl ZendType {
    /// Builds an empty Zend type container.
    ///
    /// # Parameters
    ///
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    #[must_use]
    pub fn empty(pass_by_ref: bool, is_variadic: bool) -> Self {
        Self {
            ptr: ptr::null_mut::<c_void>(),
            type_mask: Self::arg_info_flags(pass_by_ref, is_variadic),
        }
    }

    /// Attempts to create a zend type for a given datatype. Returns an option
    /// containing the type.
    ///
    /// Returns [`None`] if the data type was a class object where the class
    /// name could not be converted into a C string (i.e. contained
    /// NUL-bytes).
    ///
    /// # Parameters
    ///
    /// * `type_` - Data type to create zend type for.
    /// * `pass_by_ref` - Whether the type should be passed by reference.
    /// * `is_variadic` - Whether the type is for a variadic argument.
    /// * `allow_null` - Whether the type should allow null to be passed in
    ///   place.
    #[must_use]
    pub fn empty_from_type(
        type_: DataType,
        pass_by_ref: bool,
        is_variadic: bool,
        allow_null: bool,
    ) -> Option<Self> {
        match type_ {
            DataType::Object(Some(class)) => {
                Self::empty_from_class_type(class, pass_by_ref, is_variadic, allow_null)
            }
            type_ => Some(Self::empty_from_primitive_type(
                type_,
                pass_by_ref,
                is_variadic,
                allow_null,
            )),
        }
    }

    /// Attempts to create a zend type for a class object type. Returns an
    /// option containing the type if successful.
    ///
    /// Returns [`None`] if the data type was a class object where the class
    /// name could not be converted into a C string (i.e. contained
    /// NUL-bytes).
    ///
    /// # Parameters
    ///
    /// * `class_name` - Name of the class parameter.
    /// * `pass_by_ref` - Whether the type should be passed by reference.
    /// * `is_variadic` - Whether the type is for a variadic argument.
    /// * `allow_null` - Whether the type should allow null to be passed in
    ///   place.
    fn empty_from_class_type(
        class_name: &str,
        pass_by_ref: bool,
        is_variadic: bool,
        allow_null: bool,
    ) -> Option<Self> {
        let mut flags = Self::arg_info_flags(pass_by_ref, is_variadic);
        if allow_null {
            flags |= _ZEND_TYPE_NULLABLE_BIT;
        }
        cfg_if::cfg_if! {
            if #[cfg(php83)] {
                flags |= crate::ffi::_ZEND_TYPE_LITERAL_NAME_BIT
            } else {
                flags |= crate::ffi::_ZEND_TYPE_NAME_BIT
            }
        }

        Some(Self {
            ptr: std::ffi::CString::new(class_name)
                .ok()?
                .into_raw()
                .cast::<c_void>(),
            type_mask: flags,
        })
    }

    /// Attempts to create a zend type for a primitive PHP type.
    ///
    /// # Parameters
    ///
    /// * `type_` - Data type to create zend type for.
    /// * `pass_by_ref` - Whether the type should be passed by reference.
    /// * `is_variadic` - Whether the type is for a variadic argument.
    /// * `allow_null` - Whether the type should allow null to be passed in
    ///   place.
    ///
    /// # Panics
    ///
    /// Panics if the given `type_` is for a class object type.
    fn empty_from_primitive_type(
        type_: DataType,
        pass_by_ref: bool,
        is_variadic: bool,
        allow_null: bool,
    ) -> Self {
        assert!(!matches!(type_, DataType::Object(Some(_))));
        Self {
            ptr: ptr::null_mut::<c_void>(),
            type_mask: Self::type_init_code(type_, pass_by_ref, is_variadic, allow_null),
        }
    }

    /// Calculates the internal flags of the type.
    /// Translation of of the `_ZEND_ARG_INFO_FLAGS` macro from
    /// `zend_API.h:110`.
    ///
    /// # Parameters
    ///
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    pub(crate) fn arg_info_flags(pass_by_ref: bool, is_variadic: bool) -> u32 {
        (u32::from(pass_by_ref) << _ZEND_SEND_MODE_SHIFT)
            | (if is_variadic {
                _ZEND_IS_VARIADIC_BIT
            } else {
                0
            })
    }

    /// Calculates the internal flags of the type.
    /// Translation of the `ZEND_TYPE_INIT_CODE` macro from `zend_API.h:163`.
    ///
    /// # Parameters
    ///
    /// * `type_` - The type to initialize the Zend type with.
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    /// * `allow_null` - Whether the value can be null.
    pub(crate) fn type_init_code(
        type_: DataType,
        pass_by_ref: bool,
        is_variadic: bool,
        allow_null: bool,
    ) -> u32 {
        let type_ = type_.as_u32();

        (if type_ == _IS_BOOL {
            MAY_BE_BOOL
        } else if type_ == IS_MIXED {
            MAY_BE_ANY
        } else {
            1 << type_
        }) | (if allow_null {
            _ZEND_TYPE_NULLABLE_BIT
        } else {
            0
        }) | Self::arg_info_flags(pass_by_ref, is_variadic)
    }

    /// Converts a [`DataType`] to its `MAY_BE_*` mask value.
    ///
    /// This is used for building union types where multiple types are
    /// combined with bitwise OR in the `type_mask`.
    #[must_use]
    pub fn type_to_mask(type_: DataType) -> u32 {
        let type_val = type_.as_u32();
        if type_val == _IS_BOOL {
            MAY_BE_BOOL
        } else if type_val == IS_MIXED {
            MAY_BE_ANY
        } else {
            1 << type_val
        }
    }

    /// Creates a union type from multiple primitive data types.
    ///
    /// This method creates a PHP union type (e.g., `int|string|null`) by
    /// combining the type masks of multiple primitive types. This only
    /// supports primitive types; unions containing class types are not
    /// yet supported by this method.
    ///
    /// # Parameters
    ///
    /// * `types` - Slice of primitive data types to include in the union.
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    ///
    /// # Panics
    ///
    /// Panics if any of the types is a class object type with a class name.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ext_php_rs::flags::DataType;
    /// use ext_php_rs::zend::ZendType;
    ///
    /// // Creates `int|string` union type
    /// let union_type = ZendType::union_primitive(
    ///     &[DataType::Long, DataType::String],
    ///     false,
    ///     false,
    /// );
    ///
    /// // Creates `int|string|null` union type
    /// let nullable_union = ZendType::union_primitive(
    ///     &[DataType::Long, DataType::String, DataType::Null],
    ///     false,
    ///     false,
    /// );
    /// ```
    #[must_use]
    pub fn union_primitive(types: &[DataType], pass_by_ref: bool, is_variadic: bool) -> Self {
        let mut type_mask = Self::arg_info_flags(pass_by_ref, is_variadic);

        for type_ in types {
            assert!(
                !matches!(type_, DataType::Object(Some(_))),
                "union_primitive does not support class types"
            );
            type_mask |= Self::type_to_mask(*type_);
        }

        Self {
            ptr: ptr::null_mut::<c_void>(),
            type_mask,
        }
    }

    /// Checks if null is included in this type's mask.
    #[must_use]
    pub fn allows_null(&self) -> bool {
        // Null is allowed if either the nullable bit is set OR if the type mask
        // includes MAY_BE_NULL
        (self.type_mask & _ZEND_TYPE_NULLABLE_BIT) != 0
            || (self.type_mask & (1 << crate::ffi::IS_NULL)) != 0
    }

    /// Creates an intersection type from multiple class/interface names (PHP
    /// 8.1+).
    ///
    /// Intersection types represent a value that must satisfy ALL of the given
    /// type constraints simultaneously (e.g., `Countable&Traversable`).
    ///
    /// # Parameters
    ///
    /// * `class_names` - Slice of class/interface names that form the
    ///   intersection.
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    ///
    /// # Returns
    ///
    /// Returns `None` if any class name contains NUL bytes.
    ///
    /// # Panics
    ///
    /// Panics if fewer than 2 class names are provided.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ext_php_rs::zend::ZendType;
    ///
    /// // Creates `Countable&Traversable` intersection type
    /// let intersection = ZendType::intersection(
    ///     &["Countable", "Traversable"],
    ///     false,
    ///     false,
    /// ).unwrap();
    /// ```
    #[must_use]
    pub fn intersection(
        class_names: &[&str],
        pass_by_ref: bool,
        is_variadic: bool,
    ) -> Option<Self> {
        assert!(
            class_names.len() >= 2,
            "Intersection types require at least 2 types"
        );

        // Allocate the type list structure with space for all types
        // The zend_type_list has a flexible array member, so we need to
        // allocate extra space for the additional types.
        // We use PHP's __zend_malloc for persistent allocation so PHP can
        // properly free this memory during shutdown.
        let list_size = std::mem::size_of::<zend_type_list>()
            + (class_names.len() - 1) * std::mem::size_of::<zend_type>();

        // SAFETY: ext_php_rs_pemalloc returns properly aligned memory for any type.
        // The cast is safe because zend_type_list only requires pointer alignment.
        #[allow(clippy::cast_ptr_alignment)]
        let list_ptr = unsafe { ext_php_rs_pemalloc(list_size).cast::<zend_type_list>() };
        if list_ptr.is_null() {
            return None;
        }

        // Zero-initialize the entire allocated memory (including extra type entries)
        // This is important for PHP versions that may iterate over uninitialized
        // padding bytes
        unsafe {
            std::ptr::write_bytes(list_ptr.cast::<u8>(), 0, list_size);
        }

        // SAFETY: list_ptr is valid and properly aligned
        unsafe {
            #[allow(clippy::cast_possible_truncation)]
            {
                (*list_ptr).num_types = class_names.len() as u32;
            }

            // Get a pointer to the types array
            let types_ptr = (*list_ptr).types.as_mut_ptr();

            for (i, class_name) in class_names.iter().enumerate() {
                let type_entry = types_ptr.add(i);

                // PHP 8.3+ uses zend_string* with _ZEND_TYPE_NAME_BIT for type list entries
                // PHP < 8.3 uses const char* with _ZEND_TYPE_NAME_BIT
                cfg_if::cfg_if! {
                    if #[cfg(php83)] {
                        let zend_str = crate::ffi::ext_php_rs_zend_string_init(
                            class_name.as_ptr().cast(),
                            class_name.len(),
                            true, // persistent allocation
                        );
                        if zend_str.is_null() {
                            return None;
                        }
                        (*type_entry).ptr = zend_str.cast::<c_void>();
                        (*type_entry).type_mask = crate::ffi::_ZEND_TYPE_NAME_BIT;
                    } else {
                        let class_cstr = match std::ffi::CString::new(*class_name) {
                            Ok(s) => s,
                            Err(_) => return None,
                        };
                        (*type_entry).ptr = class_cstr.into_raw().cast::<c_void>();
                        (*type_entry).type_mask = crate::ffi::_ZEND_TYPE_NAME_BIT;
                    }
                }
            }
        }

        // Build the final type mask with intersection and list bits
        let type_mask = Self::arg_info_flags(pass_by_ref, is_variadic)
            | _ZEND_TYPE_LIST_BIT
            | _ZEND_TYPE_INTERSECTION_BIT;

        Some(Self {
            ptr: list_ptr.cast::<c_void>(),
            type_mask,
        })
    }

    /// Creates a union type containing class types (PHP 8.0+).
    ///
    /// This method creates a PHP union type where each element is a
    /// class/interface type (e.g., `Foo|Bar`). For primitive type unions,
    /// use [`Self::union_primitive`].
    ///
    /// # Parameters
    ///
    /// * `class_names` - Slice of class/interface names that form the union.
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    /// * `allow_null` - Whether null should be allowed in the union.
    ///
    /// # Returns
    ///
    /// Returns `None` if any class name contains NUL bytes.
    ///
    /// # Panics
    ///
    /// Panics if fewer than 2 class names are provided (unless `allow_null` is
    /// true, in which case 1 is acceptable).
    #[must_use]
    pub fn union_classes(
        class_names: &[&str],
        pass_by_ref: bool,
        is_variadic: bool,
        allow_null: bool,
    ) -> Option<Self> {
        let min_types = if allow_null { 1 } else { 2 };
        assert!(
            class_names.len() >= min_types,
            "Union types require at least {min_types} types"
        );

        // Allocate the type list structure using PHP's allocator
        // so PHP can properly free this memory during shutdown.
        let list_size = std::mem::size_of::<zend_type_list>()
            + (class_names.len() - 1) * std::mem::size_of::<zend_type>();

        // SAFETY: ext_php_rs_pemalloc returns properly aligned memory for any type.
        #[allow(clippy::cast_ptr_alignment)]
        let list_ptr = unsafe { ext_php_rs_pemalloc(list_size).cast::<zend_type_list>() };
        if list_ptr.is_null() {
            return None;
        }

        // Zero-initialize the entire allocated memory (including extra type entries)
        // This is important for PHP versions that may iterate over uninitialized
        // padding bytes
        unsafe {
            std::ptr::write_bytes(list_ptr.cast::<u8>(), 0, list_size);
        }

        unsafe {
            #[allow(clippy::cast_possible_truncation)]
            {
                (*list_ptr).num_types = class_names.len() as u32;
            }
            let types_ptr = (*list_ptr).types.as_mut_ptr();

            for (i, class_name) in class_names.iter().enumerate() {
                let type_entry = types_ptr.add(i);

                // PHP 8.3+ uses zend_string* with _ZEND_TYPE_NAME_BIT for type list entries
                // PHP < 8.3 uses const char* with _ZEND_TYPE_NAME_BIT
                cfg_if::cfg_if! {
                    if #[cfg(php83)] {
                        let zend_str = crate::ffi::ext_php_rs_zend_string_init(
                            class_name.as_ptr().cast(),
                            class_name.len(),
                            true, // persistent allocation
                        );
                        if zend_str.is_null() {
                            return None;
                        }
                        (*type_entry).ptr = zend_str.cast::<c_void>();
                        (*type_entry).type_mask = crate::ffi::_ZEND_TYPE_NAME_BIT;
                    } else {
                        let class_cstr = match std::ffi::CString::new(*class_name) {
                            Ok(s) => s,
                            Err(_) => return None,
                        };
                        (*type_entry).ptr = class_cstr.into_raw().cast::<c_void>();
                        (*type_entry).type_mask = crate::ffi::_ZEND_TYPE_NAME_BIT;
                    }
                }
            }
        }

        let mut type_mask = Self::arg_info_flags(pass_by_ref, is_variadic)
            | _ZEND_TYPE_LIST_BIT
            | _ZEND_TYPE_UNION_BIT;

        if allow_null {
            type_mask |= _ZEND_TYPE_NULLABLE_BIT;
        }

        Some(Self {
            ptr: list_ptr.cast::<c_void>(),
            type_mask,
        })
    }

    /// Checks if this type is an intersection type.
    #[must_use]
    pub fn is_intersection(&self) -> bool {
        (self.type_mask & _ZEND_TYPE_INTERSECTION_BIT) != 0
    }

    /// Checks if this type contains a type list (union or intersection with
    /// classes).
    #[must_use]
    pub fn has_type_list(&self) -> bool {
        (self.type_mask & _ZEND_TYPE_LIST_BIT) != 0
    }

    /// Checks if this type is a union type (excluding primitive-only unions).
    #[must_use]
    pub fn is_union(&self) -> bool {
        (self.type_mask & _ZEND_TYPE_UNION_BIT) != 0
    }

    /// Creates a DNF (Disjunctive Normal Form) type (PHP 8.2+).
    ///
    /// DNF types are unions where each element can be either a simple
    /// class/interface or an intersection group. For example:
    /// `(Countable&Traversable)|ArrayAccess`
    ///
    /// # Parameters
    ///
    /// * `groups` - Slice of type groups. Each inner slice represents either:
    ///   - A single class name (simple type)
    ///   - Multiple class names (intersection group)
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    ///
    /// # Returns
    ///
    /// Returns `None` if any class name contains NUL bytes or allocation fails.
    ///
    /// # Panics
    ///
    /// Panics if fewer than 2 groups are provided, or if any intersection group
    /// has fewer than 2 types.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ext_php_rs::zend::ZendType;
    ///
    /// // Creates `(Countable&Traversable)|ArrayAccess` DNF type
    /// let dnf = ZendType::dnf(
    ///     &[&["Countable", "Traversable"], &["ArrayAccess"]],
    ///     false,
    ///     false,
    /// ).unwrap();
    /// ```
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn dnf(groups: &[&[&str]], pass_by_ref: bool, is_variadic: bool) -> Option<Self> {
        assert!(
            groups.len() >= 2,
            "DNF types require at least 2 type groups"
        );

        // Validate: intersection groups must have at least 2 types
        for group in groups {
            if group.len() >= 2 {
                // This is an intersection group, which is valid
            } else if group.len() == 1 {
                // Single type is valid
            } else {
                panic!("Empty type group in DNF type");
            }
        }

        // Allocate the outer type list using PHP's allocator
        // so PHP can properly free this memory during shutdown.
        let outer_list_size = std::mem::size_of::<zend_type_list>()
            + (groups.len() - 1) * std::mem::size_of::<zend_type>();

        #[allow(clippy::cast_ptr_alignment)]
        let outer_list_ptr =
            unsafe { ext_php_rs_pemalloc(outer_list_size).cast::<zend_type_list>() };
        if outer_list_ptr.is_null() {
            return None;
        }

        // Zero-initialize the entire allocated memory (including extra type entries)
        // This is important for PHP versions that may iterate over uninitialized
        // padding bytes
        unsafe {
            std::ptr::write_bytes(outer_list_ptr.cast::<u8>(), 0, outer_list_size);
        }

        unsafe {
            #[allow(clippy::cast_possible_truncation)]
            {
                (*outer_list_ptr).num_types = groups.len() as u32;
            }

            let outer_types_ptr = (*outer_list_ptr).types.as_mut_ptr();

            for (i, group) in groups.iter().enumerate() {
                let type_entry = outer_types_ptr.add(i);

                if group.len() == 1 {
                    // Simple class type
                    let class_name = group[0];

                    // PHP 8.3+ uses zend_string* with _ZEND_TYPE_NAME_BIT for type list entries
                    // PHP < 8.3 uses const char* with _ZEND_TYPE_NAME_BIT
                    cfg_if::cfg_if! {
                        if #[cfg(php83)] {
                            let zend_str = crate::ffi::ext_php_rs_zend_string_init(
                                class_name.as_ptr().cast(),
                                class_name.len(),
                                true,
                            );
                            if zend_str.is_null() {
                                return None;
                            }
                            (*type_entry).ptr = zend_str.cast::<c_void>();
                            (*type_entry).type_mask = crate::ffi::_ZEND_TYPE_NAME_BIT;
                        } else {
                            let class_cstr = match std::ffi::CString::new(class_name) {
                                Ok(s) => s,
                                Err(_) => return None,
                            };
                            (*type_entry).ptr = class_cstr.into_raw().cast::<c_void>();
                            (*type_entry).type_mask = crate::ffi::_ZEND_TYPE_NAME_BIT;
                        }
                    }
                } else {
                    // Intersection group - need to create a nested type list
                    // Use PHP's allocator so PHP can properly free this memory.
                    let inner_list_size = std::mem::size_of::<zend_type_list>()
                        + (group.len() - 1) * std::mem::size_of::<zend_type>();

                    #[allow(clippy::cast_ptr_alignment)]
                    let inner_list_ptr =
                        ext_php_rs_pemalloc(inner_list_size).cast::<zend_type_list>();
                    if inner_list_ptr.is_null() {
                        return None;
                    }

                    // Zero-initialize the entire allocated memory (including extra type entries)
                    std::ptr::write_bytes(inner_list_ptr.cast::<u8>(), 0, inner_list_size);

                    #[allow(clippy::cast_possible_truncation)]
                    {
                        (*inner_list_ptr).num_types = group.len() as u32;
                    }

                    let inner_types_ptr = (*inner_list_ptr).types.as_mut_ptr();

                    for (j, class_name) in group.iter().enumerate() {
                        let inner_type_entry = inner_types_ptr.add(j);

                        cfg_if::cfg_if! {
                            if #[cfg(php83)] {
                                let zend_str = crate::ffi::ext_php_rs_zend_string_init(
                                    class_name.as_ptr().cast(),
                                    class_name.len(),
                                    true,
                                );
                                if zend_str.is_null() {
                                    return None;
                                }
                                (*inner_type_entry).ptr = zend_str.cast::<c_void>();
                                (*inner_type_entry).type_mask = crate::ffi::_ZEND_TYPE_NAME_BIT;
                            } else {
                                let class_cstr = match std::ffi::CString::new(*class_name) {
                                    Ok(s) => s,
                                    Err(_) => return None,
                                };
                                (*inner_type_entry).ptr = class_cstr.into_raw().cast::<c_void>();
                                (*inner_type_entry).type_mask = crate::ffi::_ZEND_TYPE_NAME_BIT;
                            }
                        }
                    }

                    // Set up the outer type entry to point to the intersection list
                    (*type_entry).ptr = inner_list_ptr.cast::<c_void>();
                    (*type_entry).type_mask = _ZEND_TYPE_LIST_BIT | _ZEND_TYPE_INTERSECTION_BIT;
                }
            }
        }

        // Build the final type mask with union and list bits
        let type_mask = Self::arg_info_flags(pass_by_ref, is_variadic)
            | _ZEND_TYPE_LIST_BIT
            | _ZEND_TYPE_UNION_BIT;

        Some(Self {
            ptr: outer_list_ptr.cast::<c_void>(),
            type_mask,
        })
    }
}
