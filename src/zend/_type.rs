use std::{ffi::c_void, ptr};

use crate::{
    ffi::{
        _IS_BOOL, _ZEND_IS_VARIADIC_BIT, _ZEND_SEND_MODE_SHIFT, _ZEND_TYPE_NULLABLE_BIT, IS_MIXED,
        MAY_BE_ANY, MAY_BE_BOOL, zend_type,
    },
    flags::DataType,
};

#[cfg(not(php83))]
use crate::ffi::{_ZEND_TYPE_LIST_BIT, _ZEND_TYPE_NAME_BIT, _ZEND_TYPE_UNION_BIT, zend_type_list};

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

    /// Builds a Zend type for a class union (e.g. `Foo|Bar`).
    ///
    /// On PHP 8.3+ the encoding is a single `_ZEND_TYPE_LITERAL_NAME_BIT`
    /// pointer to a NUL-terminated string of pipe-joined class names; Zend
    /// itself splits on `|`, allocates the `zend_type_list`, interns each
    /// member, and ORs `_ZEND_TYPE_LIST_BIT | _ZEND_TYPE_UNION_BIT` into the
    /// outer mask at registration time (see `Zend/zend_API.c:2929-2972` in
    /// php-src). This mirrors the single-class path's literal-name strategy.
    ///
    /// On PHP 8.1/8.2 the literal-name shortcut does not exist, so this
    /// function allocates the `zend_type_list` directly via `__zend_malloc`
    /// (matching `pemalloc(_, 1)`) and populates each entry with an interned
    /// `zend_string*` and `_ZEND_TYPE_NAME_BIT`. Zend's `zend_type_release`
    /// (see `Zend/zend_opcode.c:112-124`) handles teardown via `pefree(_, 1)`,
    /// so [`crate::zend::module::cleanup_module_allocations`] must NOT free
    /// the list itself.
    ///
    /// Returns [`None`] if `class_names` is empty or any name contains a NUL
    /// byte.
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

        let mut type_mask = Self::arg_info_flags(pass_by_ref, is_variadic);
        if allow_null {
            type_mask |= _ZEND_TYPE_NULLABLE_BIT;
        }

        cfg_if::cfg_if! {
            if #[cfg(php83)] {
                type_mask |= crate::ffi::_ZEND_TYPE_LITERAL_NAME_BIT;
                let joined = class_names.join("|");
                let ptr = std::ffi::CString::new(joined)
                    .ok()?
                    .into_raw()
                    .cast::<c_void>();
                Some(Self { ptr, type_mask })
            } else {
                build_class_union_list(class_names, type_mask)
            }
        }
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

        let mut type_mask = Self::arg_info_flags(pass_by_ref, is_variadic);
        if allow_null {
            type_mask |= _ZEND_TYPE_NULLABLE_BIT;
        }
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

/// Manually allocates a [`zend_type_list`] for a class union on PHP 8.1/8.2.
///
/// Mirrors the layout produced by Zend's own `zend_convert_internal_arg_info_type`
/// for the union-with-classes case (see `Zend/zend_API.c:2950-2970` in php-src),
/// but pushed forward to registration time so the engine sees a fully-formed
/// list immediately. On PHP 8.3+ this dance is unnecessary because Zend will
/// re-shape a `_ZEND_TYPE_LITERAL_NAME_BIT` pointer for us.
///
/// The list is allocated via `__zend_malloc` (matches `pemalloc(_, 1)`) and is
/// reclaimed by Zend's `zend_type_release` -> `pefree(_, 1)` at MSHUTDOWN
/// (`Zend/zend_opcode.c:112-124`); cleanup must NOT touch it from the Rust
/// side. Each entry holds an interned `zend_string*` (also engine-owned).
#[cfg(not(php83))]
fn build_class_union_list(class_names: &[String], outer_mask: u32) -> Option<ZendType> {
    use std::mem::size_of;

    let n = class_names.len();
    // ZEND_TYPE_LIST_SIZE(n) == sizeof(zend_type_list) + (n - 1) * sizeof(zend_type)
    // The `- 1` matches the trailing `types: [zend_type; 1]` already counted in
    // `sizeof(zend_type_list)`.
    let size = size_of::<zend_type_list>() + (n - 1) * size_of::<zend_type>();

    let raw = unsafe { crate::ffi::__zend_malloc(size) };
    if raw.is_null() {
        return None;
    }
    let list = raw.cast::<zend_type_list>();

    unsafe {
        ptr::addr_of_mut!((*list).num_types).write(u32::try_from(n).ok()?);
    }

    let entries_base = unsafe { ptr::addr_of_mut!((*list).types).cast::<zend_type>() };
    for (i, name) in class_names.iter().enumerate() {
        if name.as_bytes().contains(&0) {
            // NUL inside a class name means we'd intern garbage; abort and let
            // Zend reclaim the list at MSHUTDOWN (the outer mask still says it
            // owns a list, even though we wrote partial data).
            return None;
        }
        let zstr = crate::zend::string::intern_persistent(name);
        if zstr.is_null() {
            return None;
        }
        let entry = ZendType {
            ptr: zstr.cast::<c_void>(),
            type_mask: _ZEND_TYPE_NAME_BIT,
        };
        unsafe { entries_base.add(i).write(entry) };
    }

    Some(ZendType {
        ptr: list.cast::<c_void>(),
        type_mask: outer_mask | _ZEND_TYPE_LIST_BIT | _ZEND_TYPE_UNION_BIT,
    })
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
