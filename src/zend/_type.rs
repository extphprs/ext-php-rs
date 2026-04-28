use std::{ffi::c_void, ptr};

use crate::{
    ffi::{
        _IS_BOOL, _ZEND_IS_VARIADIC_BIT, _ZEND_SEND_MODE_SHIFT, _ZEND_TYPE_NULLABLE_BIT, IS_MIXED,
        MAY_BE_ANY, MAY_BE_BOOL, zend_type,
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
        let mut flags = Self::arg_info_flags_with_nullable(pass_by_ref, is_variadic, allow_null);
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

    /// Builds a Zend type for a class union (e.g. `Foo|Bar`).
    ///
    /// Emits a single literal-name pointer (a NUL-terminated `CString` of
    /// pipe-joined class names) and lets Zend itself split on `|`, intern
    /// each member, and rewrite the outer mask into a `zend_type_list` plus
    /// `_ZEND_TYPE_LIST_BIT | _ZEND_TYPE_UNION_BIT` at
    /// `zend_register_functions` time. The same logic exists on every
    /// supported PHP (`Zend/zend_API.c:2815-2855` on 8.1.0, `:2860-2895` on
    /// 8.2.0, `:2929-2972` on 8.3+); only the literal-name bit's *name*
    /// changes:
    ///
    /// - 8.1/8.2: `_ZEND_TYPE_NAME_BIT` itself doubles as the literal-name
    ///   bit (the engine reads `ptr` as `const char*`).
    /// - 8.3+: a dedicated `_ZEND_TYPE_LITERAL_NAME_BIT` was introduced when
    ///   `_ZEND_TYPE_NAME_BIT` shifted to mean "already-interned
    ///   `zend_string*`".
    ///
    /// Mirrors the single-class path's strategy. The `CString` is reclaimed
    /// in [`crate::zend::module::cleanup_module_allocations`].
    ///
    /// Returns [`None`] if `class_names` is empty or any name has interior
    /// NUL bytes.
    ///
    /// # Parameters
    ///
    /// * `class_names` - Class-name members of the union.
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    /// * `allow_null` - Whether the value can be null.
    #[must_use]
    pub fn empty_from_class_union(
        class_names: &[String],
        pass_by_ref: bool,
        is_variadic: bool,
        allow_null: bool,
    ) -> Option<Self> {
        if class_names.is_empty() {
            return None;
        }

        let mut type_mask =
            Self::arg_info_flags_with_nullable(pass_by_ref, is_variadic, allow_null);
        cfg_if::cfg_if! {
            if #[cfg(php83)] {
                type_mask |= crate::ffi::_ZEND_TYPE_LITERAL_NAME_BIT;
            } else {
                type_mask |= crate::ffi::_ZEND_TYPE_NAME_BIT;
            }
        }

        let joined = class_names.join("|");
        let ptr = std::ffi::CString::new(joined)
            .ok()?
            .into_raw()
            .cast::<c_void>();
        Some(Self { ptr, type_mask })
    }

    /// Builds a Zend type for a primitive union (e.g. `int|string`).
    ///
    /// PHP encodes pure primitive unions as a single [`zend_type`] whose
    /// `type_mask` ORs together the `MAY_BE_*` bits of every member; no
    /// `zend_type_list` is needed. The runtime fast-path
    /// (`zend_check_type` -> `ZEND_TYPE_CONTAINS_CODE`) reads exactly that
    /// outer mask. Lists become necessary only when class types enter the
    /// picture, which is handled by later additions.
    ///
    /// Returns [`None`] if `types` is empty (a union with zero members is
    /// malformed). Callers should pass at least two distinct member types;
    /// a single-member input is accepted but is semantically equivalent to
    /// [`Self::empty_from_type`].
    ///
    /// # Parameters
    ///
    /// * `types` - Member types of the union.
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    /// * `allow_null` - Whether the value can be null.
    #[must_use]
    pub fn empty_from_primitive_union(
        types: &[DataType],
        pass_by_ref: bool,
        is_variadic: bool,
        allow_null: bool,
    ) -> Option<Self> {
        if types.is_empty() {
            return None;
        }

        let mut type_mask =
            Self::arg_info_flags_with_nullable(pass_by_ref, is_variadic, allow_null);
        for dt in types {
            type_mask |= primitive_may_be(*dt);
        }

        Some(Self {
            ptr: ptr::null_mut(),
            type_mask,
        })
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

    /// Like [`Self::arg_info_flags`] but also threads `_ZEND_TYPE_NULLABLE_BIT`
    /// when `allow_null` is set. Centralises the pattern shared by every
    /// list/string-bearing constructor (single class, primitive union, class
    /// union); primitive scalars take a different shape via
    /// [`Self::type_init_code`].
    pub(crate) fn arg_info_flags_with_nullable(
        pass_by_ref: bool,
        is_variadic: bool,
        allow_null: bool,
    ) -> u32 {
        let mut flags = Self::arg_info_flags(pass_by_ref, is_variadic);
        if allow_null {
            flags |= _ZEND_TYPE_NULLABLE_BIT;
        }
        flags
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
}

/// Maps a [`DataType`] to its single-bit `MAY_BE_*` mask, expanding the two
/// pseudo-codes (`_IS_BOOL`, `IS_MIXED`) the same way [`ZendType::type_init_code`] does.
fn primitive_may_be(dt: DataType) -> u32 {
    let code = dt.as_u32();
    if code == _IS_BOOL {
        MAY_BE_BOOL
    } else if code == IS_MIXED {
        MAY_BE_ANY
    } else {
        1u32 << code
    }
}
