//! Builder for creating inis and methods in PHP.
//! See <https://www.phpinternalsbook.com/php7/extensions_design/ini_settings.html> for details.

use std::{ffi::CStr, os::raw::c_char, ptr};

use crate::{ffi::zend_ini_entry_def, ffi::zend_register_ini_entries, flags::IniEntryPermission};

/// A Zend ini entry definition.
///
/// Register ini definitions from your module's `startup_function` with
/// [`IniEntryDef::register`]. The table is borrowed by the engine for the life
/// of the process — PHP 8.5 stores it as `zend_ini_entry.def` and the CLI SAPI
/// reads `def->value` for `php --ini` — so it must be a `static`, exactly like
/// a C extension's `zend_ini_entry_def[]`:
///
/// ```rust,ignore
/// static INI: IniEntryDefs<2> = IniEntryDefs::new([
///     IniEntryDef::new(c"my_ext.enabled", c"1", IniEntryPermission::All),
///     IniEntryDef::end(),
/// ]);
///
/// pub fn startup(_ty: i32, mod_num: i32) -> i32 {
///     IniEntryDef::register(INI.as_slice(), mod_num);
///     0
/// }
/// ```
pub type IniEntryDef = zend_ini_entry_def;

/// A `static`-able table of [`IniEntryDef`].
///
/// [`IniEntryDef`] holds raw pointers, so an array of them is not `Sync` and
/// cannot be a `static` on its own. This wrapper is the crate's equivalent of a
/// C extension's `zend_ini_entry_def[]` in `.rodata`.
#[repr(transparent)]
pub struct IniEntryDefs<const N: usize>([IniEntryDef; N]);

// SAFETY: every pointer inside comes from a `&'static CStr`, so it stays valid
// for the life of the process. The table is immutable once constructed and the
// engine only reads through it.
unsafe impl<const N: usize> Sync for IniEntryDefs<N> {}

impl<const N: usize> IniEntryDefs<N> {
    /// Wraps an INI definition table so it can live in a `static`.
    ///
    /// The last entry must be [`IniEntryDef::end`].
    #[must_use]
    pub const fn new(entries: [IniEntryDef; N]) -> Self {
        Self(entries)
    }

    /// Returns the table as a slice, for [`IniEntryDef::register`].
    #[must_use]
    pub const fn as_slice(&self) -> &[IniEntryDef] {
        &self.0
    }
}

impl IniEntryDef {
    /// Creates a new ini entry definition.
    ///
    /// # Panics
    ///
    /// * If the name is longer than `u16::MAX` or the value longer than
    ///   `u32::MAX`.
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "IniEntryPermission is Copy and a reference is not const-friendly here"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "lengths are asserted in range above; permission bits fit in u8"
    )]
    pub const fn new(
        name: &'static CStr,
        default_value: &'static CStr,
        permission: IniEntryPermission,
    ) -> Self {
        let name_length = name.count_bytes();
        let value_length = default_value.count_bytes();
        assert!(name_length <= u16::MAX as usize, "Invalid name length");
        assert!(value_length <= u32::MAX as usize, "Invalid value length");

        let mut template = Self::end();
        template.name = name.as_ptr();
        template.name_length = name_length as u16;
        template.value = default_value.as_ptr();
        template.value_length = value_length as u32;
        template.modifiable = permission.bits() as u8;
        template
    }

    /// Returns an empty ini entry def, signifying the end of a ini list.
    #[must_use]
    pub const fn end() -> Self {
        Self {
            name: ptr::null::<c_char>(),
            on_modify: None,
            mh_arg1: std::ptr::null_mut(),
            mh_arg2: std::ptr::null_mut(),
            mh_arg3: std::ptr::null_mut(),
            value: std::ptr::null(),
            displayer: None,
            modifiable: 0,
            value_length: 0,
            name_length: 0,
        }
    }

    /// Registers a list of ini entries.
    ///
    /// `entries` must be terminated by [`IniEntryDef::end`] and must live for
    /// the whole process: `zend_register_ini_entries_ex` interns the names and
    /// values, but PHP 8.5 also keeps the table itself as
    /// `zend_ini_entry.def`.
    ///
    /// # Panics
    ///
    /// If `entries` is empty or its last element is not [`IniEntryDef::end`].
    pub fn register(entries: &'static [Self], module_number: i32) {
        assert!(
            entries.last().is_some_and(|last| last.name.is_null()),
            "INI entry tables must be terminated by `IniEntryDef::end()`"
        );

        unsafe { zend_register_ini_entries(entries.as_ptr(), module_number) };
    }
}
