//! Builder for creating inis and methods in PHP.
//! See <https://www.phpinternalsbook.com/php7/extensions_design/ini_settings.html> for details.

use std::{ffi::CString, os::raw::c_char, ptr};

use crate::{ffi::zend_ini_entry_def, ffi::zend_register_ini_entries, flags::IniEntryPermission};

/// A Zend ini entry definition.
///
/// To register ini definitions for extensions, the [`IniEntryDef`] builder
/// should be used. Ini entries should be registered in your module's
/// `startup_function` via `IniEntryDef::register(Vec<IniEntryDef>)`.
pub type IniEntryDef = zend_ini_entry_def;

impl IniEntryDef {
    /// Creates a new ini entry definition.
    ///
    /// # Panics
    ///
    /// * If the name or value cannot be converted to a C string
    /// * If the name or value length is > `65_535`
    /// * If the permission bits are invalid
    #[must_use]
    pub fn new(name: String, default_value: String, permission: &IniEntryPermission) -> Self {
        let mut template = Self::end();
        let name = CString::new(name).expect("Unable to create CString from name");
        let value = CString::new(default_value).expect("Unable to create CString from value");
        template.name_length = name
            .as_bytes()
            .len()
            .try_into()
            .expect("Invalid name length");
        template.name = name.into_raw();
        template.value_length = value
            .as_bytes()
            .len()
            .try_into()
            .expect("Invalid value length");
        template.value = value.into_raw();
        // FIXME: Double assignment of modifiable
        template.modifiable = IniEntryPermission::PerDir
            .bits()
            .try_into()
            .expect("Invalid permission bits");
        template.modifiable = permission
            .bits()
            .try_into()
            .expect("Invalid permission bits");
        template
    }

    /// Returns an empty ini entry def, signifying the end of a ini list.
    #[must_use]
    pub fn end() -> Self {
        Self {
            name: ptr::null::<c_char>(),
            on_modify: None,
            mh_arg1: std::ptr::null_mut(),
            mh_arg2: std::ptr::null_mut(),
            mh_arg3: std::ptr::null_mut(),
            value: std::ptr::null_mut(),
            displayer: None,
            modifiable: 0,
            value_length: 0,
            name_length: 0,
        }
    }

    /// Registers a list of ini entries.
    pub fn register(mut entries: Vec<Self>, module_number: i32) {
        entries.push(Self::end());
        let entries = Box::into_raw(entries.into_boxed_slice()) as *const Self;

        unsafe { zend_register_ini_entries(entries, module_number) };
    }
}
