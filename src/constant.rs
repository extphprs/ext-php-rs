//! Types and traits for registering constants in PHP.

use cfg_if::cfg_if;
use std::ffi::CString;
use std::fmt::Debug;

use super::flags::GlobalConstantFlags;
use crate::error::Result;
use crate::ffi::{
    zend_register_bool_constant, zend_register_double_constant, zend_register_long_constant,
    zend_register_string_constant,
};

/// Implemented on types which can be registered as a constant in PHP.
pub trait IntoConst: Debug {
    /// Returns the PHP stub representation of this constant value.
    ///
    /// This is used when generating PHP stub files for IDE autocompletion.
    /// The returned string should be a valid PHP literal (e.g., `"hello"`,
    /// `42`, `true`).
    fn stub_value(&self) -> String;

    /// Registers a global module constant in PHP, with the value as the content
    /// of self. This function _must_ be called in the module startup
    /// function, which is called after the module is initialized. The
    /// second parameter of the startup function will be the module number.
    /// By default, the case-insensitive and persistent flags are set when
    /// registering the constant.
    ///
    /// Returns a result containing nothing if the constant was successfully
    /// registered.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the constant.
    /// * `module_number` - The module number that we are registering the
    ///   constant under.
    ///
    /// # Errors
    ///
    /// Returns an error if the constant could not be registered.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ext_php_rs::constant::IntoConst;
    ///
    /// pub extern "C" fn startup_function(_type: i32, module_number: i32) -> i32 {
    ///     5.register_constant("MY_CONST_NAME", module_number); // MY_CONST_NAME == 5
    ///     "Hello, world!".register_constant("STRING_CONSTANT", module_number); // STRING_CONSTANT == "Hello, world!"
    ///     0
    /// }
    /// ```
    fn register_constant(&self, name: &str, module_number: i32) -> Result<()> {
        self.register_constant_flags(name, module_number, GlobalConstantFlags::Persistent)
    }

    /// Registers a global module constant in PHP, with the value as the content
    /// of self. This function _must_ be called in the module startup
    /// function, which is called after the module is initialized. The
    /// second parameter of the startup function will be the module number.
    /// This function allows you to pass any extra flags in if you require.
    /// Note that the case-sensitive and persistent flags *are not* set when you
    /// use this function, you must set these yourself.
    ///
    /// Returns a result containing nothing if the constant was successfully
    /// registered.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the constant.
    /// * `module_number` - The module number that we are registering the
    ///   constant under.
    /// * `flags` - Flags to register the constant with.
    ///
    /// # Errors
    ///
    /// Returns an error if the constant flags could not be registered.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ext_php_rs::{constant::IntoConst, flags::GlobalConstantFlags};
    ///
    /// pub extern "C" fn startup_function(_type: i32, module_number: i32) -> i32 {
    ///     42.register_constant_flags("MY_CONST_NAME", module_number, GlobalConstantFlags::Persistent | GlobalConstantFlags::Deprecated);
    ///     0
    /// }
    /// ```
    fn register_constant_flags(
        &self,
        name: &str,
        module_number: i32,
        flags: GlobalConstantFlags,
    ) -> Result<()>;
}

impl IntoConst for String {
    fn stub_value(&self) -> String {
        self.as_str().stub_value()
    }

    fn register_constant_flags(
        &self,
        name: &str,
        module_number: i32,
        flags: GlobalConstantFlags,
    ) -> Result<()> {
        self.as_str()
            .register_constant_flags(name, module_number, flags)
    }
}

impl IntoConst for &str {
    fn stub_value(&self) -> String {
        // Escape special characters for PHP string literal
        let escaped = self
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        format!("'{escaped}'")
    }

    fn register_constant_flags(
        &self,
        name: &str,
        module_number: i32,
        flags: GlobalConstantFlags,
    ) -> Result<()> {
        unsafe {
            cfg_if! {
                if #[cfg(php85)] {
                    let _ = zend_register_string_constant(
                        CString::new(name)?.as_ptr(),
                        name.len() as _,
                        CString::new(*self)?.as_ptr(),
                        flags.bits().try_into()?,
                        module_number,
                    );
                } else {
                    zend_register_string_constant(
                        CString::new(name)?.as_ptr(),
                        name.len() as _,
                        CString::new(*self)?.as_ptr(),
                        flags.bits().try_into()?,
                        module_number,
                    );
                }
            }
        };
        Ok(())
    }
}

impl IntoConst for bool {
    fn stub_value(&self) -> String {
        if *self { "true" } else { "false" }.to_string()
    }

    fn register_constant_flags(
        &self,
        name: &str,
        module_number: i32,
        flags: GlobalConstantFlags,
    ) -> Result<()> {
        unsafe {
            cfg_if! {
                if #[cfg(php85)] {
                    let _ = zend_register_bool_constant(
                        CString::new(name)?.as_ptr(),
                        name.len() as _,
                        *self,
                        flags.bits().try_into()?,
                        module_number,
                    );
                } else {
                    zend_register_bool_constant(
                        CString::new(name)?.as_ptr(),
                        name.len() as _,
                        *self,
                        flags.bits().try_into()?,
                        module_number,
                    );
                }
            }
        };
        Ok(())
    }
}

/// Implements the `IntoConst` trait for a given number type using a given
/// function.
macro_rules! into_const_num {
    ($type: ty, $fn: expr) => {
        impl IntoConst for $type {
            fn stub_value(&self) -> String {
                self.to_string()
            }

            fn register_constant_flags(
                &self,
                name: &str,
                module_number: i32,
                flags: GlobalConstantFlags,
            ) -> Result<()> {
                unsafe {
                    cfg_if! {
                        if #[cfg(php85)] {
                            let _ = $fn(
                                CString::new(name)?.as_ptr(),
                                name.len() as _,
                                (*self).into(),
                                flags.bits().try_into()?,
                                module_number,
                            );
                        } else {
                            $fn(
                                CString::new(name)?.as_ptr(),
                                name.len() as _,
                                (*self).into(),
                                flags.bits().try_into()?,
                                module_number,
                            );
                        }
                    }
                };
                Ok(())
            }
        }
    };
}

into_const_num!(i8, zend_register_long_constant);
into_const_num!(i16, zend_register_long_constant);
into_const_num!(i32, zend_register_long_constant);
into_const_num!(i64, zend_register_long_constant);
into_const_num!(f32, zend_register_double_constant);
into_const_num!(f64, zend_register_double_constant);
