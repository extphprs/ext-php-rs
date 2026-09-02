//! Types and functions used for throwing exceptions from Rust to PHP.

use std::{fmt::Debug, ptr};

use crate::{
    class::RegisteredClass,
    error::{Error, Result},
    ffi::zend_throw_exception_ex,
    ffi::zend_throw_exception_object,
    flags::ClassFlags,
    types::{ZendStr, Zval},
    zend::{ClassEntry, ce},
};

/// Result type with the error variant as a [`PhpException`].
pub type PhpResult<T = ()> = std::result::Result<T, PhpException>;

/// Represents a PHP exception which can be thrown using the `throw()` function.
/// Primarily used to return from a [`Result<T, PhpException>`] which can
/// immediately be thrown by the `ext-php-rs` macro API.
///
/// There are default [`From`] implementations for any type that implements
/// [`ToString`], so these can also be returned from these functions. You can
/// also implement [`From<T>`] for your custom error type.
#[derive(Debug)]
pub struct PhpException {
    message: String,
    code: i32,
    ex: &'static ClassEntry,
    object: Option<Zval>,
}

impl PhpException {
    /// Creates a new exception instance.
    ///
    /// # Parameters
    ///
    /// * `message` - Message to contain in the exception.
    /// * `code` - Integer code to go inside the exception.
    /// * `ex` - Exception type to throw.
    #[must_use]
    pub fn new(message: String, code: i32, ex: &'static ClassEntry) -> Self {
        Self {
            message,
            code,
            ex,
            object: None,
        }
    }

    /// Creates a new default exception instance, using the default PHP
    /// `Exception` type as the exception type, with an integer code of
    /// zero.
    ///
    /// # Parameters
    ///
    /// * `message` - Message to contain in the exception.
    #[must_use]
    pub fn default(message: String) -> Self {
        Self::new(message, 0, ce::exception())
    }

    /// Creates an instance of an exception from a PHP class type and a message.
    ///
    /// # Parameters
    ///
    /// * `message` - Message to contain in the exception.
    #[must_use]
    pub fn from_class<T: RegisteredClass>(message: String) -> Self {
        Self::new(message, 0, T::get_metadata().ce())
    }

    /// Set the Zval object for the exception.
    ///
    /// Exceptions can be based of instantiated Zval objects when you are
    /// throwing a custom exception with stateful properties.
    ///
    /// # Parameters
    ///
    /// * `object` - The Zval object.
    pub fn set_object(&mut self, object: Option<Zval>) {
        self.object = object;
    }

    /// Builder function that sets the Zval object for the exception.
    ///
    /// Exceptions can be based of instantiated Zval objects when you are
    /// throwing a custom exception with stateful properties.
    ///
    /// # Parameters
    ///
    /// * `object` - The Zval object.
    #[must_use]
    pub fn with_object(mut self, object: Zval) -> Self {
        self.object = Some(object);
        self
    }

    /// Throws the exception, returning nothing inside a result if successful
    /// and an error otherwise.
    ///
    /// # Errors
    ///
    /// * [`Error::InvalidException`] - If the exception type is an interface or
    ///   abstract class.
    /// * If the message contains NUL bytes.
    pub fn throw(self) -> Result<()> {
        match self.object {
            Some(object) => throw_object(object),
            None => throw_with_code(self.ex, self.code, &self.message),
        }
    }
}

impl From<String> for PhpException {
    fn from(str: String) -> Self {
        Self::default(str)
    }
}

impl From<&str> for PhpException {
    fn from(str: &str) -> Self {
        Self::default(str.into())
    }
}

#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for PhpException {
    fn from(err: anyhow::Error) -> Self {
        Self::new(format!("{err:#}"), 0, crate::zend::ce::exception())
    }
}

/// Throws an exception with a given message. See [`ClassEntry`] for some
/// built-in exception types.
///
/// Returns a result containing nothing if the exception was successfully
/// thrown.
///
/// # Parameters
///
/// * `ex` - The exception type to throw.
/// * `message` - The message to display when throwing the exception.
///
/// # Errors
///
/// * [`Error::InvalidException`] - If the exception type is an interface or
///   abstract class.
/// * If the message contains NUL bytes.
///
/// # Examples
///
/// ```no_run
/// use ext_php_rs::{zend::{ce, ClassEntry}, exception::throw};
///
/// throw(ce::compile_error(), "This is a CompileError.");
/// ```
pub fn throw(ex: &ClassEntry, message: &str) -> Result<()> {
    throw_with_code(ex, 0, message)
}

/// Throws an exception with a given message and status code. See [`ClassEntry`]
/// for some built-in exception types.
///
/// Returns a result containing nothing if the exception was successfully
/// thrown.
///
/// # Parameters
///
/// * `ex` - The exception type to throw.
/// * `code` - The status code to use when throwing the exception.
/// * `message` - The message to display when throwing the exception.
///
/// # Errors
///
/// * [`Error::InvalidException`] - If the exception type is an interface or
///   abstract class.
/// * If the message contains NUL bytes.
///
/// # Examples
///
/// ```no_run
/// use ext_php_rs::{zend::{ce, ClassEntry}, exception::throw_with_code};
///
/// throw_with_code(ce::compile_error(), 123, "This is a CompileError.");
/// ```
pub fn throw_with_code(ex: &ClassEntry, code: i32, message: &str) -> Result<()> {
    let flags = ex.flags();

    // Can't throw an interface or abstract class.
    if flags.contains(ClassFlags::Interface) || flags.contains(ClassFlags::Abstract) {
        return Err(Error::InvalidException(flags));
    }

    let message = ZendStr::new(message, false);
    let message_ptr = message.as_c_str()?.as_ptr();

    // SAFETY: We are given a reference to a `ClassEntry` therefore when we cast it
    // to a pointer it will be valid. `message` is NUL-terminated and outlives the
    // call; if the engine bails out here the Zend allocator reclaims it at request
    // shutdown, as php-src does for its own copy of the message.
    unsafe {
        zend_throw_exception_ex(
            ptr::from_ref(ex).cast_mut(),
            code.into(),
            c"%s".as_ptr(),
            message_ptr,
        )
    };
    Ok(())
}

/// Throws an exception object.
///
/// Returns a result containing nothing if the exception was successfully
/// thrown.
///
/// # Parameters
///
/// * `object` - The zval of type object
///
/// # Errors
///
/// *shrug*
/// TODO: does this error?
///
/// # Examples
///
/// ```no_run
/// use ext_php_rs::prelude::*;
/// use ext_php_rs::exception::throw_object;
/// use crate::ext_php_rs::convert::IntoZval;
///
/// #[php_class]
/// #[php(extends(ce = ext_php_rs::zend::ce::exception, stub = "\\Exception"))]
/// pub struct JsException {
///     #[php(prop, flags = ext_php_rs::flags::PropertyFlags::Public)]
///     message: String,
///     #[php(prop, flags = ext_php_rs::flags::PropertyFlags::Public)]
///     code: i32,
///     #[php(prop, flags = ext_php_rs::flags::PropertyFlags::Public)]
///     file: String,
/// }
///
/// #[php_module]
/// pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
///     module
/// }
///
/// let error = JsException { message: "A JS error occurred.".to_string(), code: 100, file: "index.js".to_string() };
/// throw_object( error.into_zval(true).unwrap() );
/// ```
pub fn throw_object(zval: Zval) -> Result<()> {
    let mut zv = core::mem::ManuallyDrop::new(zval);
    unsafe { zend_throw_exception_object(core::ptr::addr_of_mut!(zv).cast()) };
    Ok(())
}

#[cfg(feature = "embed")]
#[cfg(test)]
mod tests {
    #![allow(clippy::assertions_on_constants)]
    use super::*;
    use crate::embed::Embed;

    #[test]
    fn test_new() {
        Embed::run(|| {
            let ex = PhpException::new("Test".into(), 0, ce::exception());
            assert_eq!(ex.message, "Test");
            assert_eq!(ex.code, 0);
            assert_eq!(ex.ex, ce::exception());
            assert!(ex.object.is_none());
        });
    }

    #[test]
    fn test_default() {
        Embed::run(|| {
            let ex = PhpException::default("Test".into());
            assert_eq!(ex.message, "Test");
            assert_eq!(ex.code, 0);
            assert_eq!(ex.ex, ce::exception());
            assert!(ex.object.is_none());
        });
    }

    #[test]
    fn test_set_object() {
        Embed::run(|| {
            let mut ex = PhpException::default("Test".into());
            assert!(ex.object.is_none());
            let obj = Zval::new();
            ex.set_object(Some(obj));
            assert!(ex.object.is_some());
        });
    }

    #[test]
    fn test_with_object() {
        Embed::run(|| {
            let obj = Zval::new();
            let ex = PhpException::default("Test".into()).with_object(obj);
            assert!(ex.object.is_some());
        });
    }

    #[test]
    fn test_throw_code() {
        Embed::run(|| {
            let ex = PhpException::default("Test".into());
            assert!(ex.throw().is_ok());

            assert!(false, "Should not reach here");
        });
    }

    #[test]
    fn test_throw_object() {
        Embed::run(|| {
            let ex = PhpException::default("Test".into()).with_object(Zval::new());
            assert!(ex.throw().is_ok());

            assert!(false, "Should not reach here");
        });
    }

    #[test]
    fn test_from_string() {
        Embed::run(|| {
            let ex: PhpException = "Test".to_string().into();
            assert_eq!(ex.message, "Test");
            assert_eq!(ex.code, 0);
            assert_eq!(ex.ex, ce::exception());
            assert!(ex.object.is_none());
        });
    }

    #[test]
    fn test_from_str() {
        Embed::run(|| {
            let ex: PhpException = "Test str".into();
            assert_eq!(ex.message, "Test str");
            assert_eq!(ex.code, 0);
            assert_eq!(ex.ex, ce::exception());
            assert!(ex.object.is_none());
        });
    }

    #[cfg(feature = "anyhow")]
    #[test]
    fn test_from_anyhow() {
        Embed::run(|| {
            let ex: PhpException = anyhow::anyhow!("Test anyhow").into();
            assert_eq!(ex.message, "Test anyhow");
            assert_eq!(ex.code, 0);
            assert_eq!(ex.ex, ce::exception());
            assert!(ex.object.is_none());
        });
    }

    #[test]
    fn test_throw_ex() {
        Embed::run(|| {
            assert!(throw(ce::exception(), "Test").is_ok());

            assert!(false, "Should not reach here");
        });
    }

    #[test]
    fn test_throw_with_code() {
        Embed::run(|| {
            assert!(throw_with_code(ce::exception(), 1, "Test").is_ok());

            assert!(false, "Should not reach here");
        });
    }

    // TODO: Test abstract class
    #[test]
    fn test_throw_with_code_interface() {
        Embed::run(|| {
            assert!(throw_with_code(ce::arrayaccess(), 0, "Test").is_err());
        });
    }

    #[test]
    fn test_static_throw_object() {
        Embed::run(|| {
            let obj = Zval::new();
            assert!(throw_object(obj).is_ok());

            assert!(false, "Should not reach here");
        });
    }
}
