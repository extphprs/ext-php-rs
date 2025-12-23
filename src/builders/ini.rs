use crate::embed::ext_php_rs_php_ini_builder_deinit;
use crate::ffi::{
    php_ini_builder, php_ini_builder_define, php_ini_builder_prepend, php_ini_builder_quoted,
    php_ini_builder_unquoted,
};
use crate::util::CStringScope;
use std::ffi::{CStr, NulError};
use std::fmt::Display;
use std::ops::Deref;

/// A builder for creating INI configurations.
pub type IniBuilder = php_ini_builder;

impl Default for IniBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IniBuilder {
    /// Creates a new INI builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ext_php_rs::builders::IniBuilder;
    /// let mut builder = IniBuilder::new();
    /// ```
    #[must_use]
    pub fn new() -> IniBuilder {
        IniBuilder {
            value: std::ptr::null_mut(),
            length: 0,
        }
    }

    /// Appends a value to the INI builder.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to append.
    ///
    /// # Errors
    ///
    /// Returns a `NulError` if the value contains a null byte.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ext_php_rs::builders::IniBuilder;
    /// let mut builder = IniBuilder::new();
    /// builder.prepend("foo=bar");
    /// ```
    pub fn prepend<V: AsRef<str>>(&mut self, value: V) -> Result<(), NulError> {
        let value = value.as_ref();
        let raw = CStringScope::new(value)?;

        unsafe {
            php_ini_builder_prepend(self, *raw, value.len());
        }

        Ok(())
    }

    /// Appends an unquoted name-value pair to the INI builder.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the pair.
    /// * `value` - The value of the pair.
    ///
    /// # Errors
    ///
    /// Returns a `NulError` if the value contains a null byte.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ext_php_rs::builders::IniBuilder;
    /// let mut builder = IniBuilder::new();
    /// builder.unquoted("foo", "bar");
    /// ```
    pub fn unquoted<N, V>(&mut self, name: N, value: V) -> Result<(), NulError>
    where
        N: AsRef<str>,
        V: AsRef<str>,
    {
        let name = name.as_ref();
        let value = value.as_ref();

        let raw_name = CStringScope::new(name)?;
        let raw_value = CStringScope::new(value)?;

        unsafe {
            php_ini_builder_unquoted(self, *raw_name, name.len(), *raw_value, value.len());
        }

        Ok(())
    }

    /// Appends a quoted name-value pair to the INI builder.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the pair.
    /// * `value` - The value of the pair.
    ///
    /// # Errors
    ///
    /// Returns a `NulError` if the value contains a null byte.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ext_php_rs::builders::IniBuilder;
    /// let mut builder = IniBuilder::new();
    /// builder.quoted("foo", "bar");
    /// ```
    pub fn quoted<N, V>(&mut self, name: N, value: V) -> Result<(), NulError>
    where
        N: AsRef<str>,
        V: AsRef<str>,
    {
        let name = name.as_ref();
        let value = value.as_ref();

        let raw_name = CStringScope::new(name)?;
        let raw_value = CStringScope::new(value)?;

        unsafe {
            php_ini_builder_quoted(self, *raw_name, name.len(), *raw_value, value.len());
        }

        Ok(())
    }

    /// Defines a value in the INI builder.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to define.
    ///
    /// # Errors
    ///
    /// Returns a `NulError` if the value contains a null byte.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ext_php_rs::builders::IniBuilder;
    /// let mut builder = IniBuilder::new();
    /// builder.define("foo=bar");
    /// ```
    pub fn define<V: AsRef<str>>(&mut self, value: V) -> Result<(), NulError> {
        let value = value.as_ref();
        let raw = CStringScope::new(value)?;

        unsafe {
            php_ini_builder_define(self, *raw);
        }

        Ok(())
    }

    /// Finishes building the INI configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ext_php_rs::builders::IniBuilder;
    /// let mut builder = IniBuilder::new();
    /// let ini = builder.finish();
    /// ```
    #[must_use]
    pub fn finish(&self) -> &CStr {
        unsafe {
            if self.value.is_null() {
                return c"";
            }

            self.value.add(self.length).write(0);
            CStr::from_ptr(self.value)
        }
    }
}

impl Display for IniBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content: &str = self.as_ref();
        write!(f, "{content}")
    }
}

impl Deref for IniBuilder {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl From<IniBuilder> for String {
    fn from(builder: IniBuilder) -> Self {
        let temp: &str = builder.as_ref();
        temp.to_string()
    }
}

impl AsRef<CStr> for IniBuilder {
    fn as_ref(&self) -> &CStr {
        self.finish()
    }
}

impl AsRef<str> for IniBuilder {
    fn as_ref(&self) -> &str {
        self.finish().to_str().unwrap_or("")
    }
}

// Ensure the C buffer is properly deinitialized when the builder goes out of
// scope.
impl Drop for IniBuilder {
    fn drop(&mut self) {
        if !self.value.is_null() {
            unsafe {
                ext_php_rs_php_ini_builder_deinit(self);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ini_builder_prepend() {
        let mut builder = IniBuilder::new();
        builder.prepend("foo=bar").expect("should prepend value");

        let ini = builder.finish();
        assert!(!ini.is_empty());
        assert_eq!(ini, c"foo=bar");
    }

    #[test]
    fn test_ini_builder_unquoted() {
        let mut builder = IniBuilder::new();
        builder
            .unquoted("baz", "qux")
            .expect("should add unquoted value");

        let ini = builder.finish();
        assert!(!ini.is_empty());
        assert_eq!(ini, c"baz=qux\n");
    }

    #[test]
    fn test_ini_builder_quoted() {
        let mut builder = IniBuilder::new();
        builder
            .quoted("quux", "corge")
            .expect("should add quoted value");

        let ini = builder.finish();
        assert!(!ini.is_empty());
        assert_eq!(ini, c"quux=\"corge\"\n");
    }

    #[test]
    fn test_ini_builder_define() {
        let mut builder = IniBuilder::new();
        builder
            .define("grault=garply")
            .expect("should define value");

        let ini = builder.finish();
        assert!(!ini.is_empty());
        assert_eq!(ini, c"grault=garply\n");
    }

    #[test]
    fn test_ini_builder_null_byte_error() {
        let mut builder = IniBuilder::new();
        assert!(builder.prepend("key=val\0ue").is_err());
    }

    #[test]
    fn test_ini_builder_empty_values() {
        let mut builder = IniBuilder::new();
        assert!(builder.unquoted("", "").is_ok());
    }
}
