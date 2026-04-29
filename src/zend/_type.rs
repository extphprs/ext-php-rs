use std::{ffi::c_void, ptr};

#[cfg(php83)]
use crate::types::DnfTerm;
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
    /// * `allow_null` - Whether the value can be null. Must be `false`;
    ///   `true` returns [`None`] (the legal nullable form is the DNF
    ///   `(Foo&Bar)|null`, build a [`PhpType::Dnf`] for that).
    ///
    /// # Version constraint
    ///
    /// Available on PHP 8.3+ only. `ReflectionIntersectionType` was
    /// introduced in PHP 8.1, but `zend_register_functions` on 8.1/8.2
    /// rejects pre-built `zend_type_list` for internal-function `arg_info`
    /// (`Zend/zend_API.c` insists on `ZEND_TYPE_HAS_NAME` and re-parses
    /// from a literal `const char*`; the engine only splits on `|`, not
    /// `&`, so an `&`-joined literal name is not a viable encoding
    /// either). 8.3+ added the `ZEND_TYPE_HAS_LITERAL_NAME` check that
    /// leaves pre-built lists alone.
    ///
    /// [`PhpType::Dnf`]: crate::types::PhpType::Dnf
    #[cfg(php83)]
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

        let list_ptr = build_class_list(class_names)?;

        let type_mask = Self::arg_info_flags(pass_by_ref, is_variadic)
            | crate::ffi::_ZEND_TYPE_LIST_BIT
            | crate::ffi::_ZEND_TYPE_INTERSECTION_BIT
            | crate::ffi::_ZEND_TYPE_ARENA_BIT;

        Some(Self {
            ptr: list_ptr.cast::<c_void>(),
            type_mask,
        })
    }

    /// Builds a Zend type for a DNF (Disjunctive Normal Form) type
    /// (e.g. `(A&B)|C`). PHP 8.2+.
    ///
    /// DNF is a top-level union whose alternatives may themselves be class
    /// intersection groups. The on-disk shape mirrors what `zend_compile.c`
    /// produces for `(A&B)|C`:
    ///
    /// 1. An outer [`zend_type_list`](crate::ffi::zend_type_list) with one
    ///    entry per [`DnfTerm`].
    /// 2. Each [`DnfTerm::Single`] becomes a list entry whose `ptr` is a
    ///    persistent-interned `zend_string*` and whose mask is
    ///    `_ZEND_TYPE_NAME_BIT`.
    /// 3. Each [`DnfTerm::Intersection`] becomes a nested
    ///    [`zend_type_list`](crate::ffi::zend_type_list) (allocated and
    ///    populated identically to a flat
    ///    [`Self::empty_from_class_intersection`]); the corresponding outer
    ///    list entry's `ptr` points at that inner list and its mask carries
    ///    `_ZEND_TYPE_LIST_BIT | _ZEND_TYPE_INTERSECTION_BIT |
    ///    _ZEND_TYPE_ARENA_BIT`.
    /// 4. The outer mask carries `_ZEND_TYPE_LIST_BIT |
    ///    _ZEND_TYPE_UNION_BIT | _ZEND_TYPE_ARENA_BIT`, plus
    ///    `_ZEND_TYPE_NULLABLE_BIT` when `allow_null` is set.
    ///
    /// The arena bit on every list (outer and inner) tells Zend's recursive
    /// `zend_type_release` (`Zend/zend_opcode.c:112-124`) to skip the
    /// `pefree` of our hand-allocations. Each `zend_string` is tagged
    /// `IS_STR_INTERNED` by
    /// [`crate::ffi::ext_php_rs_zend_string_init_persistent_interned`] so
    /// `zend_string_release` becomes a no-op and the strings survive embed
    /// MSHUTDOWN cycles. Lists and strings live for the process lifetime
    /// (one allocation set per DNF arg/retval per module — bounded leak,
    /// reclaimed at `DL_UNLOAD`). The
    /// [`_ZEND_TYPE_LIST_BIT`](crate::ffi::_ZEND_TYPE_LIST_BIT) skip in
    /// [`crate::zend::module::cleanup_module_allocations`] already covers
    /// every level of this nested layout.
    ///
    /// Returns [`None`] when:
    ///
    /// - `terms` is empty,
    /// - `terms.len() == 1` (degenerate; use [`PhpType::Simple`] for a
    ///   single class or [`PhpType::Intersection`] for a flat intersection),
    /// - any [`DnfTerm::Intersection`] carries fewer than 2 members,
    /// - any class name is empty or contains an interior NUL byte, or
    /// - allocation fails.
    ///
    /// [`PhpType::Simple`]: crate::types::PhpType::Simple
    /// [`PhpType::Intersection`]: crate::types::PhpType::Intersection
    ///
    /// # Parameters
    ///
    /// * `terms` - Class-side disjuncts in declaration order.
    /// * `pass_by_ref` - Whether the value should be passed by reference.
    /// * `is_variadic` - Whether this type represents a variadic argument.
    /// * `allow_null` - Whether the value can be null. Threads the
    ///   `_ZEND_TYPE_NULLABLE_BIT` on the outer mask; this is the canonical
    ///   way to spell `(A&B)|null`.
    ///
    /// # Version constraint
    ///
    /// Available on PHP 8.3+ only. PHP 8.2 introduced DNF in user code but
    /// its `zend_register_functions` does not accept pre-built
    /// `zend_type_list` for internal-function `arg_info` (same root cause as
    /// [`Self::empty_from_class_intersection`]); the engine only began
    /// honouring `_ZEND_TYPE_LIST_BIT` here in 8.3+ via the
    /// `ZEND_TYPE_HAS_LITERAL_NAME` gate.
    #[cfg(php83)]
    #[must_use]
    pub fn empty_from_dnf(
        terms: &[DnfTerm],
        pass_by_ref: bool,
        is_variadic: bool,
        allow_null: bool,
    ) -> Option<Self> {
        if terms.len() < 2 {
            // Empty or single-term DNF is degenerate — callers should pick
            // the more specific variant (Simple, ClassUnion, Intersection)
            // explicitly. Refusing here keeps a single canonical spelling
            // per legal PHP type.
            return None;
        }

        for term in terms {
            if !dnf_term_is_valid(term) {
                return None;
            }
        }

        let num_terms = u32::try_from(terms.len()).ok()?;

        let outer_size = std::mem::size_of::<crate::ffi::zend_type_list>()
            + (terms.len().saturating_sub(1)) * std::mem::size_of::<zend_type>();

        // SAFETY: pemalloc(_, 1). Arena bit on the outer mask below tells
        // Zend's `zend_type_release` to skip the `pefree` of this list, so
        // the allocation lives for the process lifetime.
        let outer_list = unsafe { crate::ffi::ext_php_rs_pemalloc_persistent(outer_size) }
            .cast::<crate::ffi::zend_type_list>();

        if outer_list.is_null() {
            return None;
        }

        // SAFETY: `outer_list` points to a freshly-allocated
        // `zend_type_list` with capacity for `num_terms` entries.
        unsafe {
            (*outer_list).num_types = num_terms;
        }

        for (i, term) in terms.iter().enumerate() {
            let entry = match term {
                DnfTerm::Single(name) => {
                    let s = unsafe {
                        crate::ffi::ext_php_rs_zend_string_init_persistent_interned(
                            name.as_ptr().cast::<i8>(),
                            name.len(),
                        )
                    };
                    if s.is_null() {
                        return None;
                    }
                    zend_type {
                        ptr: s.cast::<c_void>(),
                        type_mask: crate::ffi::_ZEND_TYPE_NAME_BIT,
                    }
                }
                DnfTerm::Intersection(names) => {
                    let inner_list = build_class_list(names)?;
                    zend_type {
                        ptr: inner_list.cast::<c_void>(),
                        type_mask: crate::ffi::_ZEND_TYPE_LIST_BIT
                            | crate::ffi::_ZEND_TYPE_INTERSECTION_BIT
                            | crate::ffi::_ZEND_TYPE_ARENA_BIT,
                    }
                }
            };

            // SAFETY: `types` is a flexible array; index `i` is within the
            // freshly-allocated capacity (`num_terms` entries).
            unsafe {
                let slot = (*outer_list).types.as_mut_ptr().add(i);
                *slot = entry;
            }
        }

        let type_mask = Self::arg_info_flags_with_nullable(pass_by_ref, is_variadic, allow_null)
            | crate::ffi::_ZEND_TYPE_LIST_BIT
            | crate::ffi::_ZEND_TYPE_UNION_BIT
            | crate::ffi::_ZEND_TYPE_ARENA_BIT;

        Some(Self {
            ptr: outer_list.cast::<c_void>(),
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

/// Allocates and populates a `zend_type_list` for a sequence of class names.
///
/// Shared between [`ZendType::empty_from_class_intersection`] and
/// [`ZendType::empty_from_dnf`] (both PHP 8.3+) — DNF nests one of these
/// lists per intersection group. The caller owns the bit flags on the outer
/// `zend_type` that points at this list; this helper only handles the list
/// itself and its entries.
///
/// Returns [`None`] when `class_names` is empty, any name has interior NUL
/// bytes or is empty, or allocation fails. Each entry is tagged
/// `_ZEND_TYPE_NAME_BIT` with a persistent-interned `zend_string*`
/// (allocated via
/// [`crate::ffi::ext_php_rs_zend_string_init_persistent_interned`], which
/// sets `IS_STR_INTERNED` so `zend_string_release` becomes a no-op).
///
/// The engine processes our pre-built list directly in
/// `zend_register_functions` — `Zend/zend_API.c` 8.3+ uses
/// `ZEND_TYPE_HAS_LITERAL_NAME` to decide whether to re-parse a literal
/// name, leaving `_ZEND_TYPE_LIST_BIT`-bearing types alone. PHP 8.1/8.2
/// instead used `ZEND_TYPE_IS_COMPLEX` and asserted `HAS_NAME`, so this
/// pre-built shape would crash at registration time on those versions.
/// The caller is responsible for setting `_ZEND_TYPE_ARENA_BIT` on the
/// parent mask so Zend's recursive `zend_type_release` skips the `pefree`
/// of the list itself.
#[cfg(php83)]
fn build_class_list(class_names: &[String]) -> Option<*mut crate::ffi::zend_type_list> {
    if class_names.is_empty() {
        return None;
    }

    for name in class_names {
        if name.is_empty() || name.as_bytes().contains(&0u8) {
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

    // SAFETY: Allocates with `pemalloc(_, 1)`. The caller sets the arena
    // bit on the parent mask so Zend's `zend_type_release` skips the
    // `pefree` of this list.
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

    Some(list_ptr)
}

/// Returns `true` when the given DNF term is a legal shape:
/// `Single` carries a non-empty NUL-free name; `Intersection` carries 2 or
/// more such names. One-element intersection groups are rejected to keep a
/// single canonical Rust spelling per legal PHP type.
#[cfg(php83)]
fn dnf_term_is_valid(term: &DnfTerm) -> bool {
    match term {
        DnfTerm::Single(name) => !name.is_empty() && !name.as_bytes().contains(&0u8),
        DnfTerm::Intersection(names) => {
            names.len() >= 2
                && names
                    .iter()
                    .all(|n| !n.is_empty() && !n.as_bytes().contains(&0u8))
        }
    }
}

#[cfg(all(test, php83))]
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

#[cfg(all(test, php83))]
mod dnf_tests {
    use super::*;
    use crate::ffi::{
        _ZEND_TYPE_ARENA_BIT, _ZEND_TYPE_INTERSECTION_BIT, _ZEND_TYPE_LIST_BIT,
        _ZEND_TYPE_NAME_BIT, _ZEND_TYPE_NULLABLE_BIT, _ZEND_TYPE_UNION_BIT, zend_type_list,
    };

    fn dnf_a_and_b_or_c() -> Vec<DnfTerm> {
        vec![
            DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
            DnfTerm::Single("C".to_owned()),
        ]
    }

    #[test]
    fn empty_from_dnf_sets_outer_list_union_arena_bits() {
        let terms = dnf_a_and_b_or_c();
        let ty = ZendType::empty_from_dnf(&terms, false, false, false).expect("DNF should build");

        assert_ne!(ty.type_mask & _ZEND_TYPE_LIST_BIT, 0);
        assert_ne!(ty.type_mask & _ZEND_TYPE_UNION_BIT, 0);
        assert_ne!(
            ty.type_mask & _ZEND_TYPE_ARENA_BIT,
            0,
            "arena bit must be set on the outer DNF list",
        );
        assert_eq!(
            ty.type_mask & _ZEND_TYPE_INTERSECTION_BIT,
            0,
            "outer DNF list is a union, not an intersection",
        );
        assert_eq!(ty.type_mask & _ZEND_TYPE_NULLABLE_BIT, 0);
        assert!(!ty.ptr.is_null());

        let list = ty.ptr.cast::<zend_type_list>();
        let num = unsafe { (*list).num_types };
        assert_eq!(num, 2);
    }

    #[test]
    fn empty_from_dnf_intersection_term_has_list_intersection_arena_bits() {
        let terms = dnf_a_and_b_or_c();
        let ty = ZendType::empty_from_dnf(&terms, false, false, false).expect("DNF should build");

        let list = ty.ptr.cast::<zend_type_list>();
        let entry0 = unsafe { *(*list).types.as_ptr() };

        assert_ne!(entry0.type_mask & _ZEND_TYPE_LIST_BIT, 0);
        assert_ne!(entry0.type_mask & _ZEND_TYPE_INTERSECTION_BIT, 0);
        assert_ne!(
            entry0.type_mask & _ZEND_TYPE_ARENA_BIT,
            0,
            "inner intersection list must also carry the arena bit",
        );
        assert!(!entry0.ptr.is_null());
    }

    #[test]
    fn empty_from_dnf_single_class_term_has_name_bit_only() {
        let terms = dnf_a_and_b_or_c();
        let ty = ZendType::empty_from_dnf(&terms, false, false, false).expect("DNF should build");

        let list = ty.ptr.cast::<zend_type_list>();
        let entry1 = unsafe { *(*list).types.as_ptr().add(1) };

        assert_ne!(entry1.type_mask & _ZEND_TYPE_NAME_BIT, 0);
        assert_eq!(
            entry1.type_mask & _ZEND_TYPE_LIST_BIT,
            0,
            "single-class term is not a list",
        );
        assert!(!entry1.ptr.is_null(), "must hold a zend_string*");
    }

    #[test]
    fn empty_from_dnf_with_allow_null_sets_nullable_bit() {
        let terms = dnf_a_and_b_or_c();
        let ty = ZendType::empty_from_dnf(&terms, false, false, true)
            .expect("nullable DNF should build");

        assert_ne!(
            ty.type_mask & _ZEND_TYPE_NULLABLE_BIT,
            0,
            "allow_null must propagate _ZEND_TYPE_NULLABLE_BIT",
        );
    }

    #[test]
    fn empty_from_dnf_two_intersection_terms() {
        let terms = vec![
            DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
            DnfTerm::Intersection(vec!["C".to_owned(), "D".to_owned()]),
        ];
        let ty = ZendType::empty_from_dnf(&terms, false, false, false)
            .expect("(A&B)|(C&D) should build");

        let list = ty.ptr.cast::<zend_type_list>();
        for i in 0..2 {
            let entry = unsafe { *(*list).types.as_ptr().add(i) };
            assert_ne!(entry.type_mask & _ZEND_TYPE_LIST_BIT, 0);
            assert_ne!(entry.type_mask & _ZEND_TYPE_INTERSECTION_BIT, 0);
            assert_ne!(entry.type_mask & _ZEND_TYPE_ARENA_BIT, 0);
        }
    }

    #[test]
    fn empty_from_dnf_rejects_empty_terms() {
        assert!(ZendType::empty_from_dnf(&[], false, false, false).is_none());
    }

    #[test]
    fn empty_from_dnf_rejects_single_class_only() {
        let terms = vec![DnfTerm::Single("C".to_owned())];
        assert!(
            ZendType::empty_from_dnf(&terms, false, false, false).is_none(),
            "single-class DNF should be rejected (use PhpType::Simple)",
        );
    }

    #[test]
    fn empty_from_dnf_rejects_single_intersection_only() {
        let terms = vec![DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()])];
        assert!(
            ZendType::empty_from_dnf(&terms, false, false, false).is_none(),
            "single-intersection DNF should be rejected (use PhpType::Intersection)",
        );
    }

    #[test]
    fn empty_from_dnf_rejects_intersection_with_one_member() {
        let terms = vec![
            DnfTerm::Intersection(vec!["A".to_owned()]),
            DnfTerm::Single("C".to_owned()),
        ];
        assert!(
            ZendType::empty_from_dnf(&terms, false, false, false).is_none(),
            "single-element intersection group should be rejected",
        );
    }

    #[test]
    fn empty_from_dnf_rejects_interior_nul() {
        let terms = vec![
            DnfTerm::Intersection(vec!["A".to_owned(), "B\0".to_owned()]),
            DnfTerm::Single("C".to_owned()),
        ];
        assert!(ZendType::empty_from_dnf(&terms, false, false, false).is_none());

        let terms = vec![
            DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
            DnfTerm::Single("C\0".to_owned()),
        ];
        assert!(ZendType::empty_from_dnf(&terms, false, false, false).is_none());
    }

    #[test]
    fn empty_from_dnf_rejects_empty_class_name() {
        let terms = vec![
            DnfTerm::Intersection(vec!["A".to_owned(), String::new()]),
            DnfTerm::Single("C".to_owned()),
        ];
        assert!(ZendType::empty_from_dnf(&terms, false, false, false).is_none());

        let terms = vec![
            DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
            DnfTerm::Single(String::new()),
        ];
        assert!(ZendType::empty_from_dnf(&terms, false, false, false).is_none());
    }
}
