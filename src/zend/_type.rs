use std::{ffi::c_void, ptr};

#[cfg(php82)]
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
    /// 1. Allocate a `zend_type_list` with `pemalloc(_, 1)` (via
    ///    `ext_php_rs_pemalloc_persistent`, which hides the file/line
    ///    parameters that vary between debug and release builds).
    /// 2. For each class name, allocate a persistent `zend_string` tagged
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

        let list_ptr = build_class_list(class_names, true)?;

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
                    let inner_list = build_class_list(names, true)?;
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

    /// Builds a Zend type suitable for `zend_declare_typed_property`.
    ///
    /// Property registration is structurally distinct from `arg_info` on every
    /// supported PHP version: `zend_declare_typed_property` stores the
    /// `zend_type` verbatim, with no `zend_register_functions`-style literal
    /// name preprocessing. Class names must therefore reach the engine as
    /// `zend_string*` (not `const char*` literals), and class unions must be
    /// pre-built `zend_type_list`s instead of pipe-joined literals.
    /// php-src's own `gen_stub.php` emits this shape on every supported
    /// version (`build/gen_stub.php` 8.1 line 1450, 8.2 line 2194, master
    /// line 2419).
    ///
    /// Lifecycle: every allocation here is engine-managed. Strings are
    /// refcounted persistent (no `IS_STR_INTERNED`); `zend_type_list`s carry
    /// no `_ZEND_TYPE_ARENA_BIT`. At internal-class destroy (MSHUTDOWN), the
    /// engine's `zend_type_release` (`Zend/zend_opcode.c:112-124`) walks the
    /// shape and `pefree`s the list + `zend_string_release`s each entry.
    /// Mirrors the per-MINIT allocation rhythm: every cycle re-builds the
    /// shape against a fresh class entry, every MSHUTDOWN releases it. No
    /// accumulating leak in embed tests; no `cleanup_module_allocations`
    /// involvement (that hook is `arg_info`-only).
    ///
    /// # Version constraints
    ///
    /// - `Simple` (primitive or class), `Union`, `ClassUnion`: every
    ///   supported version. Properties accept these on 8.0+; the engine
    ///   surface for typed properties exists since the language feature
    ///   landed.
    /// - `Intersection`: PHP 8.1+ (language minimum). Returns [`None`] on
    ///   earlier versions.
    /// - `Dnf`: PHP 8.3+ (the language feature is 8.2 but
    ///   `zend_declare_typed_property` only accepts the nested intersection
    ///   terms from 8.3 onwards). Returns [`None`] on earlier versions.
    ///
    /// Differs from the `arg_info` `cfg(php83)` gate on intersection / DNF:
    /// `zend_declare_typed_property` accepts pre-built `zend_type_list`s on
    /// every version that supports the language feature, whereas
    /// `zend_register_functions` did not until 8.3.
    ///
    /// # Returns
    ///
    /// [`None`] when:
    ///
    /// - any class name is empty or contains an interior NUL byte,
    /// - allocation fails,
    /// - the variant is `Intersection` on PHP < 8.1 or `Dnf` on PHP < 8.3,
    /// - the variant is empty (e.g. `ClassUnion(vec![])`),
    /// - the variant is structurally degenerate per its constructor's rules
    ///   (e.g. single-term DNF — see [`Self::empty_from_dnf`] for the
    ///   canonical-spelling rationale, mirrored here for property symmetry).
    ///
    /// # Parameters
    ///
    /// * `ty` - The PHP type to build for.
    /// * `allow_null` - Whether the property accepts `null`. Combined with
    ///   the type's nullability rules.
    #[must_use]
    pub fn empty_for_property(ty: &crate::types::PhpType, allow_null: bool) -> Option<Self> {
        use crate::types::PhpType;

        match ty {
            PhpType::Simple(DataType::Object(Some(class))) => {
                Self::empty_from_class_for_property(class, allow_null)
            }
            PhpType::Simple(dt) => Some(Self {
                ptr: ptr::null_mut(),
                type_mask: Self::type_init_code(*dt, false, false, allow_null),
            }),
            PhpType::Union(members) => {
                let mut type_mask = if allow_null {
                    _ZEND_TYPE_NULLABLE_BIT
                } else {
                    0
                };
                for dt in members {
                    type_mask |= primitive_may_be(*dt);
                }
                Some(Self {
                    ptr: ptr::null_mut(),
                    type_mask,
                })
            }
            PhpType::ClassUnion(class_names) => {
                Self::empty_from_class_union_for_property(class_names, allow_null)
            }
            PhpType::Intersection(class_names) => {
                Self::empty_from_class_intersection_for_property(class_names, allow_null)
            }
            PhpType::Dnf(terms) => Self::empty_from_dnf_for_property(terms, allow_null),
        }
    }

    /// Property-side single class builder. Emits a `zend_string*`-bearing
    /// `zend_type` (mask = `_ZEND_TYPE_NAME_BIT [| _ZEND_TYPE_NULLABLE_BIT]`)
    /// instead of the literal-name shape used by [`Self::empty_from_class_type`]
    /// for `arg_info`, because `zend_declare_typed_property` does no
    /// literal-name preprocessing on any version.
    ///
    /// The string is allocated via
    /// [`crate::ffi::ext_php_rs_zend_string_init`] with `persistent = true`,
    /// so the engine takes ownership and refcount-releases it at
    /// internal-class destroy.
    ///
    /// Returns [`None`] on empty / interior-NUL class name or allocation
    /// failure.
    fn empty_from_class_for_property(class_name: &str, allow_null: bool) -> Option<Self> {
        if class_name.is_empty() || class_name.as_bytes().contains(&0u8) {
            return None;
        }

        let str_ptr = unsafe {
            crate::ffi::ext_php_rs_zend_string_init(
                class_name.as_ptr().cast::<i8>(),
                class_name.len(),
                true,
            )
        };
        if str_ptr.is_null() {
            return None;
        }

        let mut type_mask = crate::ffi::_ZEND_TYPE_NAME_BIT;
        if allow_null {
            type_mask |= _ZEND_TYPE_NULLABLE_BIT;
        }

        Some(Self {
            ptr: str_ptr.cast::<c_void>(),
            type_mask,
        })
    }

    /// Property-side class union builder. Allocates a real `zend_type_list`
    /// with one `_ZEND_TYPE_NAME_BIT` + `zend_string*` entry per member, then
    /// wraps it with `_ZEND_TYPE_LIST_BIT | _ZEND_TYPE_UNION_BIT [|
    /// _ZEND_TYPE_NULLABLE_BIT]`. No arena bit — the engine `pefree`s the
    /// list at internal-class destroy.
    ///
    /// Mirrors `gen_stub.php`'s property emission for `Foo|Bar`:
    /// `ZEND_TYPE_INIT_UNION(<list>, MAY_BE_NULL?)`.
    fn empty_from_class_union_for_property(
        class_names: &[String],
        allow_null: bool,
    ) -> Option<Self> {
        if class_names.is_empty() {
            return None;
        }

        let list_ptr = build_class_list(class_names, false)?;

        let mut type_mask = crate::ffi::_ZEND_TYPE_LIST_BIT | crate::ffi::_ZEND_TYPE_UNION_BIT;
        if allow_null {
            type_mask |= _ZEND_TYPE_NULLABLE_BIT;
        }

        Some(Self {
            ptr: list_ptr.cast::<c_void>(),
            type_mask,
        })
    }

    /// Property-side class intersection builder (PHP 8.1+).
    ///
    /// Same shape as the `arg_info` intersection but without the `_ZEND_TYPE_ARENA_BIT`
    /// (engine reclaims the list) and with non-interned strings (engine refcount-releases).
    /// `allow_null` is rejected; nullable intersections must be expressed as
    /// `(A&B)|null` via [`PhpType::Dnf`](crate::types::PhpType::Dnf).
    #[cfg(php81)]
    fn empty_from_class_intersection_for_property(
        class_names: &[String],
        allow_null: bool,
    ) -> Option<Self> {
        if class_names.is_empty() || allow_null {
            return None;
        }

        let list_ptr = build_class_list(class_names, false)?;

        let type_mask = crate::ffi::_ZEND_TYPE_LIST_BIT | crate::ffi::_ZEND_TYPE_INTERSECTION_BIT;

        Some(Self {
            ptr: list_ptr.cast::<c_void>(),
            type_mask,
        })
    }

    /// Property-side intersection on pre-8.1 returns `None`.
    #[cfg(not(php81))]
    fn empty_from_class_intersection_for_property(
        _class_names: &[String],
        _allow_null: bool,
    ) -> Option<Self> {
        None
    }

    /// Property-side DNF builder (PHP 8.3+).
    ///
    /// Same nested-list shape as the `arg_info` DNF but without `_ZEND_TYPE_ARENA_BIT`
    /// at every level (the engine's recursive `zend_type_release` frees the
    /// inner intersection lists), and with non-interned strings. Gated at
    /// PHP 8.3 because 8.2's `zend_declare_typed_property` iterates the type
    /// list with `ZEND_ASSERT(!ZEND_TYPE_HAS_LIST(*single_type))`, rejecting
    /// the nested intersection terms a DNF embeds. PHP 8.3 dropped that
    /// assertion in favour of `zend_normalize_internal_type`.
    #[cfg(php83)]
    fn empty_from_dnf_for_property(terms: &[DnfTerm], allow_null: bool) -> Option<Self> {
        if terms.len() < 2 {
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

        // SAFETY: pemalloc(_, 1). No arena bit on the outer mask below, so
        // Zend's `zend_type_release` will `pefree` this list at internal-class
        // destroy.
        let outer_list = unsafe { crate::ffi::ext_php_rs_pemalloc_persistent(outer_size) }
            .cast::<crate::ffi::zend_type_list>();

        if outer_list.is_null() {
            return None;
        }

        // SAFETY: `outer_list` points to a freshly-allocated `zend_type_list`
        // with capacity for `num_terms` entries.
        unsafe {
            (*outer_list).num_types = num_terms;
        }

        for (i, term) in terms.iter().enumerate() {
            let entry = match term {
                DnfTerm::Single(name) => {
                    let s = unsafe {
                        crate::ffi::ext_php_rs_zend_string_init(
                            name.as_ptr().cast::<i8>(),
                            name.len(),
                            true,
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
                    let inner_list = build_class_list(names, false)?;
                    zend_type {
                        ptr: inner_list.cast::<c_void>(),
                        type_mask: crate::ffi::_ZEND_TYPE_LIST_BIT
                            | crate::ffi::_ZEND_TYPE_INTERSECTION_BIT,
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

        let mut type_mask = crate::ffi::_ZEND_TYPE_LIST_BIT | crate::ffi::_ZEND_TYPE_UNION_BIT;
        if allow_null {
            type_mask |= _ZEND_TYPE_NULLABLE_BIT;
        }

        Some(Self {
            ptr: outer_list.cast::<c_void>(),
            type_mask,
        })
    }

    /// Property-side DNF on pre-8.3 returns `None`.
    #[cfg(not(php83))]
    fn empty_from_dnf_for_property(
        _terms: &[crate::types::DnfTerm],
        _allow_null: bool,
    ) -> Option<Self> {
        None
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
        let type_ = crate::flags::data_type_as_u32(&type_);

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
    let code = crate::flags::data_type_as_u32(&dt);
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
/// Used by both the `arg_info` path
/// ([`ZendType::empty_from_class_intersection`] / [`ZendType::empty_from_dnf`],
/// PHP 8.3+) and the property path ([`ZendType::empty_from_class_union_for_property`]
/// and friends, PHP 8.1+ depending on variant). DNF nests one of these lists
/// per intersection group. The caller owns the bit flags on the outer
/// `zend_type` that points at this list; this helper only handles the list
/// itself and its entries.
///
/// `interned` selects the lifetime model:
///
/// - `true` (`arg_info`): each `zend_string` is allocated via
///   [`crate::ffi::ext_php_rs_zend_string_init_persistent_interned`], which
///   sets `IS_STR_INTERNED` so `zend_string_release` becomes a no-op. The
///   caller MUST set `_ZEND_TYPE_ARENA_BIT` on the parent mask so Zend's
///   `zend_type_release` (`Zend/zend_opcode.c:112-124`) skips the `pefree` of
///   the list. Net effect: the list and strings live for the process
///   lifetime — needed because `#[php_module]` caches function entries
///   across embed-test re-init cycles, and the engine would otherwise free
///   the list-owned strings out from under our cached `arg_info`.
/// - `false` (property): each `zend_string` is allocated via
///   [`crate::ffi::ext_php_rs_zend_string_init`] with `persistent = true`,
///   producing a refcounted persistent string. The caller MUST NOT set
///   `_ZEND_TYPE_ARENA_BIT`; Zend's `zend_type_release` then `pefree`s the
///   list and `zend_string_release`s each entry at internal-class destroy
///   (MSHUTDOWN). Property registration runs through `ClassBuilder::register`
///   per MINIT against a fresh class entry, so the engine-managed cleanup
///   matches the per-cycle allocation lifetime — no accumulating leak in
///   embed tests, no double-free, mirrors what php-src `gen_stub.php` emits
///   for typed property declarations on every supported version.
///
/// The engine processes our pre-built list directly: in
/// `zend_register_functions` 8.3+ via `ZEND_TYPE_HAS_LITERAL_NAME` (`arg_info`),
/// and in `zend_declare_typed_property` on every supported version
/// (property). PHP 8.1/8.2's `zend_register_functions` rejected pre-built
/// lists for `arg_info` — that's why the `cfg(php83)` gate stays on the
/// `arg_info` callers — but `zend_declare_typed_property` accepts them on 8.1+.
///
/// Returns [`None`] when `class_names` is empty, any name has interior NUL
/// bytes or is empty, or allocation fails. Each entry is tagged
/// `_ZEND_TYPE_NAME_BIT` regardless of `interned`; only the underlying
/// `zend_string` allocator differs.
fn build_class_list(
    class_names: &[String],
    interned: bool,
) -> Option<*mut crate::ffi::zend_type_list> {
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

    // SAFETY: Allocates with `pemalloc(_, 1)`. With `interned = true`, the
    // caller sets the arena bit on the parent mask so Zend's
    // `zend_type_release` skips the `pefree` of this list. With
    // `interned = false`, Zend `pefree`s this allocation at class destroy.
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
            if interned {
                crate::ffi::ext_php_rs_zend_string_init_persistent_interned(
                    name.as_ptr().cast::<i8>(),
                    name.len(),
                )
            } else {
                crate::ffi::ext_php_rs_zend_string_init(
                    name.as_ptr().cast::<i8>(),
                    name.len(),
                    true,
                )
            }
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
#[cfg(php82)]
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

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::ffi::{_ZEND_TYPE_LIST_BIT, _ZEND_TYPE_NULLABLE_BIT, IS_LONG, IS_STRING};
    use crate::types::PhpType;

    #[cfg(feature = "embed")]
    use crate::ffi::{
        _ZEND_TYPE_ARENA_BIT, _ZEND_TYPE_NAME_BIT, _ZEND_TYPE_UNION_BIT, zend_type_list,
    };

    fn may_be_long() -> u32 {
        1u32 << IS_LONG
    }

    fn may_be_string() -> u32 {
        1u32 << IS_STRING
    }

    #[test]
    fn empty_for_property_simple_primitive_emits_type_mask_only() {
        let ty = ZendType::empty_for_property(&PhpType::Simple(DataType::Long), false)
            .expect("simple primitive should build");

        assert!(ty.ptr.is_null(), "primitive must not carry a pointer");
        assert_ne!(ty.type_mask & may_be_long(), 0);
        assert_eq!(
            ty.type_mask & _ZEND_TYPE_LIST_BIT,
            0,
            "primitive must not set the list bit",
        );
        assert_eq!(ty.type_mask & _ZEND_TYPE_NULLABLE_BIT, 0);
    }

    #[test]
    fn empty_for_property_nullable_primitive_sets_nullable_bit() {
        let ty = ZendType::empty_for_property(&PhpType::Simple(DataType::Long), true)
            .expect("nullable primitive should build");

        assert_ne!(ty.type_mask & _ZEND_TYPE_NULLABLE_BIT, 0);
        assert_ne!(ty.type_mask & may_be_long(), 0);
    }

    #[test]
    fn empty_for_property_primitive_union_ors_may_be_bits() {
        let ty = ZendType::empty_for_property(
            &PhpType::Union(vec![DataType::Long, DataType::String]),
            false,
        )
        .expect("primitive union should build");

        assert!(ty.ptr.is_null(), "primitive union must not carry a pointer");
        assert_ne!(ty.type_mask & may_be_long(), 0);
        assert_ne!(ty.type_mask & may_be_string(), 0);
        assert_eq!(ty.type_mask & _ZEND_TYPE_LIST_BIT, 0);
    }

    #[test]
    #[cfg(feature = "embed")]
    fn empty_for_property_class_emits_name_bit_with_zend_string() {
        crate::embed::Embed::run(|| {
            let ty = ZendType::empty_for_property(
                &PhpType::Simple(DataType::Object(Some("Foo"))),
                false,
            )
            .expect("class should build");

            assert_ne!(
                ty.type_mask & _ZEND_TYPE_NAME_BIT,
                0,
                "single class property must carry _ZEND_TYPE_NAME_BIT",
            );
            assert_eq!(
                ty.type_mask & _ZEND_TYPE_LIST_BIT,
                0,
                "single class property must not set the list bit",
            );
            assert_eq!(ty.type_mask & _ZEND_TYPE_NULLABLE_BIT, 0);
            assert!(
                !ty.ptr.is_null(),
                "single class property must hold a zend_string pointer",
            );
        });
    }

    #[test]
    #[cfg(feature = "embed")]
    fn empty_for_property_class_union_builds_list_without_arena() {
        crate::embed::Embed::run(|| {
            let ty = ZendType::empty_for_property(
                &PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()]),
                false,
            )
            .expect("class union property should build");

            assert_ne!(ty.type_mask & _ZEND_TYPE_LIST_BIT, 0);
            assert_ne!(ty.type_mask & _ZEND_TYPE_UNION_BIT, 0);
            assert_eq!(
                ty.type_mask & _ZEND_TYPE_ARENA_BIT,
                0,
                "property class union must NOT set the arena bit (engine-managed cleanup)",
            );

            let list = ty.ptr.cast::<zend_type_list>();
            let num = unsafe { (*list).num_types };
            assert_eq!(num, 2);

            for i in 0..2 {
                let entry = unsafe { *(*list).types.as_ptr().add(i) };
                assert_ne!(entry.type_mask & _ZEND_TYPE_NAME_BIT, 0);
                assert!(!entry.ptr.is_null());
            }
        });
    }

    #[test]
    #[cfg(all(feature = "embed", php81))]
    fn empty_for_property_class_intersection_no_arena() {
        crate::embed::Embed::run(|| {
            let ty = ZendType::empty_for_property(
                &PhpType::Intersection(vec!["Countable".to_owned(), "Traversable".to_owned()]),
                false,
            )
            .expect("intersection property should build on 8.1+");

            assert_ne!(ty.type_mask & _ZEND_TYPE_LIST_BIT, 0);
            assert_ne!(ty.type_mask & crate::ffi::_ZEND_TYPE_INTERSECTION_BIT, 0,);
            assert_eq!(
                ty.type_mask & _ZEND_TYPE_ARENA_BIT,
                0,
                "property intersection must NOT set the arena bit",
            );
        });
    }

    #[test]
    #[cfg(all(feature = "embed", php82))]
    fn empty_for_property_dnf_no_arena_at_any_level() {
        crate::embed::Embed::run(|| {
            let terms = vec![
                DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
                DnfTerm::Single("C".to_owned()),
            ];
            let ty = ZendType::empty_for_property(&PhpType::Dnf(terms), false)
                .expect("DNF property should build on 8.2+");

            assert_ne!(ty.type_mask & _ZEND_TYPE_LIST_BIT, 0);
            assert_ne!(ty.type_mask & _ZEND_TYPE_UNION_BIT, 0);
            assert_eq!(
                ty.type_mask & _ZEND_TYPE_ARENA_BIT,
                0,
                "outer DNF list must NOT set arena bit",
            );

            let list = ty.ptr.cast::<zend_type_list>();
            let entry0 = unsafe { *(*list).types.as_ptr() };
            assert_ne!(entry0.type_mask & _ZEND_TYPE_LIST_BIT, 0);
            assert_ne!(
                entry0.type_mask & crate::ffi::_ZEND_TYPE_INTERSECTION_BIT,
                0,
            );
            assert_eq!(
                entry0.type_mask & _ZEND_TYPE_ARENA_BIT,
                0,
                "inner intersection list must NOT set arena bit",
            );

            let entry1 = unsafe { *(*list).types.as_ptr().add(1) };
            assert_ne!(entry1.type_mask & _ZEND_TYPE_NAME_BIT, 0);
            assert_eq!(
                entry1.type_mask & _ZEND_TYPE_LIST_BIT,
                0,
                "single-class DNF term is not a list",
            );
        });
    }

    #[test]
    fn empty_for_property_rejects_empty_class_union() {
        let ty = ZendType::empty_for_property(&PhpType::ClassUnion(vec![]), false);
        assert!(ty.is_none(), "empty class union must be rejected");
    }

    #[test]
    fn empty_for_property_rejects_empty_class_name() {
        let ty = ZendType::empty_for_property(&PhpType::Simple(DataType::Object(Some(""))), false);
        assert!(ty.is_none(), "empty class name must be rejected");
    }

    #[cfg(not(php81))]
    #[test]
    fn empty_for_property_intersection_returns_none_pre_81() {
        let ty = ZendType::empty_for_property(
            &PhpType::Intersection(vec!["A".to_owned(), "B".to_owned()]),
            false,
        );
        assert!(ty.is_none(), "intersection property is 8.1+");
    }

    #[cfg(not(php83))]
    #[test]
    fn empty_for_property_dnf_returns_none_pre_83() {
        use crate::types::DnfTerm;
        let terms = vec![
            DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
            DnfTerm::Single("C".to_owned()),
        ];
        let ty = ZendType::empty_for_property(&PhpType::Dnf(terms), false);
        assert!(ty.is_none(), "DNF property registration is 8.3+");
    }
}
