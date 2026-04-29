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

    /// Builds a Zend type for a class intersection (e.g. `Foo&Bar`).
    ///
    /// Unlike [`Self::empty_from_class_union`], the literal-name shortcut
    /// does NOT work for intersections. Verified across PHP 8.1.34, 8.2.30,
    /// 8.3.30, 8.4.20, 8.5.5 and master: `zend_convert_internal_arg_info_type`
    /// only ever splits on `|`. There is no `&` parsing path, so the engine
    /// will never rewrite an `&`-joined literal name into an
    /// `_ZEND_TYPE_INTERSECTION_BIT` list at registration time.
    ///
    /// Instead, this constructor hand-rolls the same shape `gen_stub.php`
    /// emits for property/argument intersection types (see
    /// `Zend/ext/zend_test/test_arginfo.h:1363-1370` in php-src):
    ///
    /// 1. Allocate a [`zend_type_list`] with `pemalloc(_, 1)` (via
    ///    `ext_php_rs_pemalloc_persistent`, which hides the file/line
    ///    parameters that vary between debug and release builds).
    /// 2. For each class name, allocate a persistent [`zend_string`] tagged
    ///    with `IS_STR_INTERNED` (via
    ///    `ext_php_rs_zend_string_init_persistent_interned`). The interned
    ///    flag turns Zend's `zend_string_release` into a no-op so the
    ///    strings survive every MSHUTDOWN cycle, which matters because the
    ///    `#[php_module]` macro caches our function entries across embed
    ///    test re-init. Calling the real `zend_string_init_interned`
    ///    (function pointer wired up mid-startup) from `get_module()`
    ///    crashed the issue 02 first attempt; setting the flag on a plain
    ///    `zend_string_init` allocation has no lifecycle dependency.
    /// 3. Populate each list entry with `_ZEND_TYPE_NAME_BIT` and the
    ///    just-allocated `zend_string*`.
    /// 4. Set `_ZEND_TYPE_LIST_BIT | _ZEND_TYPE_INTERSECTION_BIT |
    ///    _ZEND_TYPE_ARENA_BIT` on the outer `type_mask`. The arena bit
    ///    tells Zend's `zend_type_release` (`Zend/zend_opcode.c:112-124`)
    ///    to skip the `pefree` of the list itself, leaving lifecycle to us
    ///    so the list survives the engine's startup/shutdown cycles too.
    ///
    /// Net effect: the list and its strings are persistently allocated
    /// once during `get_module()` and live for the process lifetime. The
    /// existing `_ZEND_TYPE_LIST_BIT` skip in
    /// [`crate::zend::module::cleanup_module_allocations`] is already
    /// correct: we leak the allocations on purpose (the leak is bounded —
    /// one list + N strings per intersection type per module, freed by
    /// the OS when the process exits).
    ///
    /// Returns [`None`] when:
    ///
    /// - `class_names` is empty,
    /// - any class name has an interior NUL byte (NUL would terminate the C
    ///   string Zend later inspects), or
    /// - `allow_null` is `true`. PHP user code cannot spell `?Foo&Bar`; the
    ///   only legal form is the DNF `(Foo&Bar)|null` which is the
    ///   responsibility of the future DNF representation. This constructor
    ///   refuses nullable intersections so callers fail early instead of
    ///   silently producing a half-built type.
    ///
    /// # Parameters
    ///
    /// * `class_names` - Class-name members of the intersection.
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    /// * `allow_null` - Whether the value can be null. Must be `false` in
    ///   slice 03; `true` returns [`None`].
    #[cfg(php81)]
    #[must_use]
    pub fn empty_from_class_intersection(
        class_names: &[String],
        pass_by_ref: bool,
        is_variadic: bool,
        allow_null: bool,
    ) -> Option<Self> {
        if class_names.is_empty() || allow_null {
            return None;
        }

        for name in class_names {
            if name.as_bytes().contains(&0u8) {
                return None;
            }
        }

        let num_types = u32::try_from(class_names.len()).ok()?;

        // SAFETY: Layout matches Zend's `ZEND_TYPE_LIST_SIZE(num_types)` macro
        // (`Zend/zend_types.h`). The `types` field is a flexible array
        // member declared as `[zend_type; 1]`, so the struct already
        // accounts for one entry; remaining entries are tail-allocated.
        let list_size = std::mem::size_of::<crate::ffi::zend_type_list>()
            + (class_names.len().saturating_sub(1)) * std::mem::size_of::<zend_type>();

        // SAFETY: Allocates with `pemalloc(_, 1)`. The arena bit set on the
        // outer mask below tells Zend's `zend_type_release` to skip the
        // `pefree` of this list, so the allocation lives for the process
        // lifetime (one list per intersection arg/retval per module).
        let list_ptr = unsafe { crate::ffi::ext_php_rs_pemalloc_persistent(list_size) }
            .cast::<crate::ffi::zend_type_list>();

        if list_ptr.is_null() {
            return None;
        }

        // SAFETY: `list_ptr` points to a freshly-allocated `zend_type_list`
        // with capacity for `num_types` entries.
        unsafe {
            (*list_ptr).num_types = num_types;
        }

        for (i, name) in class_names.iter().enumerate() {
            let str_ptr = unsafe {
                crate::ffi::ext_php_rs_zend_string_init_persistent_interned(
                    name.as_ptr().cast::<i8>(),
                    name.len(),
                )
            };
            if str_ptr.is_null() {
                // No teardown needed: Zend will reclaim the partially-built
                // list and any strings already attached when the module
                // fails to load (the outer caller propagates None as an
                // `Error::InvalidCString`).
                return None;
            }

            // SAFETY: `types` is a flexible array; index `i` is within the
            // freshly-allocated capacity (num_types entries).
            unsafe {
                let entry = (*list_ptr).types.as_mut_ptr().add(i);
                *entry = zend_type {
                    ptr: str_ptr.cast::<c_void>(),
                    type_mask: crate::ffi::_ZEND_TYPE_NAME_BIT,
                };
            }
        }

        let type_mask = Self::arg_info_flags(pass_by_ref, is_variadic)
            | crate::ffi::_ZEND_TYPE_LIST_BIT
            | crate::ffi::_ZEND_TYPE_INTERSECTION_BIT
            | crate::ffi::_ZEND_TYPE_ARENA_BIT;

        Some(Self {
            ptr: list_ptr.cast::<c_void>(),
            type_mask,
        })
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

#[cfg(all(test, php81))]
mod intersection_tests {
    use super::*;
    use crate::ffi::{
        _ZEND_TYPE_ARENA_BIT, _ZEND_TYPE_INTERSECTION_BIT, _ZEND_TYPE_LIST_BIT,
        _ZEND_TYPE_NAME_BIT, _ZEND_TYPE_NULLABLE_BIT, zend_type_list,
    };

    #[test]
    fn empty_from_class_intersection_sets_list_intersection_and_arena_bits() {
        let names = vec!["Countable".to_owned(), "Traversable".to_owned()];
        let ty = ZendType::empty_from_class_intersection(&names, false, false, false)
            .expect("intersection should build");

        assert_ne!(ty.type_mask & _ZEND_TYPE_LIST_BIT, 0);
        assert_ne!(ty.type_mask & _ZEND_TYPE_INTERSECTION_BIT, 0);
        assert_ne!(
            ty.type_mask & _ZEND_TYPE_ARENA_BIT,
            0,
            "arena bit must be set so Zend keeps its hands off the list"
        );
        assert_eq!(ty.type_mask & _ZEND_TYPE_NULLABLE_BIT, 0);
        assert!(!ty.ptr.is_null());

        let list = ty.ptr.cast::<zend_type_list>();
        let num = unsafe { (*list).num_types };
        assert_eq!(num, 2);
    }

    #[test]
    fn empty_from_class_intersection_rejects_nullable() {
        let names = vec!["Countable".to_owned(), "Traversable".to_owned()];
        let ty = ZendType::empty_from_class_intersection(&names, false, false, true);
        assert!(
            ty.is_none(),
            "nullable intersection should be rejected (DNF in slice 04)"
        );
    }

    #[test]
    fn empty_from_class_intersection_rejects_empty() {
        let names: Vec<String> = vec![];
        let ty = ZendType::empty_from_class_intersection(&names, false, false, false);
        assert!(ty.is_none(), "empty intersection should be rejected");
    }

    #[test]
    fn empty_from_class_intersection_rejects_interior_nul() {
        let names = vec!["Foo".to_owned(), "B\0ar".to_owned()];
        let ty = ZendType::empty_from_class_intersection(&names, false, false, false);
        assert!(ty.is_none(), "names with NUL bytes should be rejected");
    }

    #[test]
    fn empty_from_class_intersection_marks_each_entry_as_name_bit() {
        let names = vec!["Foo".to_owned(), "Bar".to_owned()];
        let ty = ZendType::empty_from_class_intersection(&names, false, false, false)
            .expect("intersection should build");

        let list = ty.ptr.cast::<zend_type_list>();
        let entries = unsafe { (*list).types.as_ptr() };
        for i in 0..2 {
            let entry = unsafe { *entries.add(i) };
            assert_ne!(
                entry.type_mask & _ZEND_TYPE_NAME_BIT,
                0,
                "entry {i} must carry _ZEND_TYPE_NAME_BIT"
            );
            assert!(!entry.ptr.is_null(), "entry {i} must hold a zend_string*");
        }
    }
}
