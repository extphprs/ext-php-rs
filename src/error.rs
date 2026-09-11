//! Error and result types returned from the library functions.

use std::{
    ffi::{CString, NulError, c_int},
    num::TryFromIntError,
};

use crate::{
    exception::PhpException,
    ffi::php_error_docref,
    flags::{ClassFlags, DataType, ErrorType, ZvalTypeFlags},
    zend::{CatchError, ExecutorGlobals},
};

/// The main result type which is passed by the library.
pub type Result<T, E = Error> = std::result::Result<T, E>;

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<Error>();
};

const _: () = assert!(
    size_of::<Error>() == size_of::<String>() + size_of::<usize>(),
    "Error is returned by every Result in the crate, including the conversion paths: a variant that grows it past a String payload is paid for on every call"
);

/// The main error type which is passed by the library inside the custom
/// [`Result`] type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An incorrect number of arguments was given to a PHP function.
    ///
    /// The enum carries two integers - the first representing the number of
    /// arguments that were received, and the second representing the minimum
    /// number of arguments expected.
    #[error("expected at least {1} arguments, got {0} arguments")]
    IncorrectArguments(usize, usize),
    /// There was an error converting a Zval into a primitive type.
    ///
    /// The enum carries the data type of the Zval.
    #[error("could not convert value of type {0}")]
    ZvalConversion(DataType),
    /// The function type read from the engine is not one this crate models.
    ///
    /// The enum carries the raw value read from `zend_function.type`.
    #[error("unknown function type {0}")]
    UnknownFunctionType(u8),
    /// Attempted to convert a [`ZvalTypeFlags`] struct to a [`DataType`].
    /// The flags did not contain a datatype.
    ///
    /// The enum carries the flags that were attempted to be converted to a
    /// [`DataType`].
    #[error("type flags did not contain a datatype: {0:?}")]
    InvalidTypeToDatatype(ZvalTypeFlags),
    /// The function called was called in an invalid scope (calling
    /// class-related functions inside of a non-class bound function).
    #[error("invalid scope")]
    InvalidScope,
    /// The pointer inside a given type was invalid, either null or pointing to
    /// garbage.
    #[error("invalid pointer")]
    InvalidPointer,
    /// The given property name does not exist.
    #[error("property `{property}` does not exist on the object")]
    InvalidProperty {
        /// Name of the property that was looked up.
        property: String,
    },
    /// The enum has no case matching the given name or discriminant.
    #[error("enum has no case matching `{case}`")]
    InvalidEnumCase {
        /// Name or discriminant that was looked up.
        case: String,
    },
    /// The string could not be converted into a C-string due to the presence of
    /// a NUL character.
    #[error("string contains a NUL byte at position {position}")]
    InvalidCString {
        /// Byte offset of the first NUL character.
        position: usize,
    },
    /// The string could not be converted into a valid Utf8 string
    #[error("invalid UTF-8 byte sequence")]
    InvalidUtf8,
    /// Could not call the given function.
    #[error("could not call the given function")]
    Callable,
    /// An object was expected.
    #[error("an object was expected")]
    Object,
    /// The object's class does not implement `__toString()`, so it cannot be
    /// converted into a string.
    ///
    /// The enum carries the name of the class.
    #[error("{0} does not implement __toString()")]
    NotStringable(String),
    /// An invalid exception type was thrown.
    #[error("invalid exception type was thrown: {0:?}")]
    InvalidException(ClassFlags),
    /// Converting integer arguments resulted in an overflow.
    #[error("converting integer arguments resulted in an overflow")]
    IntegerOverflow,
    /// The engine did not run a [`try_catch`] closure to completion.
    ///
    /// [`try_catch`]: crate::zend::try_catch
    #[error("try_catch did not run the closure to completion")]
    Catch(#[from] CatchError),
    /// A PHP exception is pending in the executor globals.
    ///
    /// The exception is left where the engine put it, so it propagates on its
    /// own once control returns to PHP. Take ownership of it with
    /// [`ExecutorGlobals::take_exception`] to handle it from Rust.
    ///
    /// The enum carries the class name of the pending exception.
    #[error("a PHP exception of class {class} is pending")]
    ExceptionPending {
        /// Class name of the pending exception.
        class: String,
    },
    /// A failure occurred while registering the stream wrapper
    #[error("a failure occurred while registering the stream wrapper")]
    StreamWrapperRegistrationFailure,
    /// A failure occurred while unregistering the stream wrapper
    #[error("a failure occurred while unregistering the stream wrapper")]
    StreamWrapperUnregistrationFailure,
    /// The SAPI write function is not available
    #[error("the SAPI write function is not available")]
    SapiWriteUnavailable,
    /// Failed to make an object lazy (PHP 8.4+)
    #[error("failed to make the object lazy")]
    LazyObjectFailed,
    /// The engine failed to load an auto-global.
    ///
    /// The enum carries the name of the auto-global.
    #[error("the engine failed to load the `{0}` auto-global")]
    AutoGlobalLoadFailed(&'static str),
}

impl Error {
    /// Builds an [`Error::ExceptionPending`] from the exception the engine is
    /// currently holding, leaving the exception where it is.
    ///
    /// Returns [`None`] when no exception is pending.
    pub(crate) fn pending_exception() -> Option<Self> {
        ExecutorGlobals::pending_exception_class().map(|class| Self::ExceptionPending { class })
    }
}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Self {
        Self::IntegerOverflow
    }
}

impl From<NulError> for Error {
    fn from(value: NulError) -> Self {
        Self::InvalidCString {
            position: value.nul_position(),
        }
    }
}

impl From<Error> for PhpException {
    fn from(err: Error) -> Self {
        Self::from_message(err.to_string())
    }
}

/// Trigger an error that is reported in PHP the same way `trigger_error()` is.
///
/// See specific error type descriptions at <https://www.php.net/manual/en/errorfunc.constants.php>.
///
/// Does nothing if `message` contains a NUL byte, or if the error type bits do not
/// fit in a C `int`.
pub fn php_error(type_: &ErrorType, message: &str) {
    let Ok(c_string) = CString::new(message) else {
        return;
    };
    let Ok(bits) = c_int::try_from(type_.bits()) else {
        return;
    };

    // SAFETY: `php_error_docref` is declared `PHP_ATTRIBUTE_FORMAT(printf, 3, 4)`, so
    // `message` must be passed as a `%s` argument and never as the format itself,
    // which would interpret `%` sequences in it as varargs directives. Both pointers
    // are NUL-terminated and outlive the call.
    unsafe {
        php_error_docref(std::ptr::null(), bits, c"%s".as_ptr(), c_string.as_ptr());
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    #[test]
    fn exception_pending_names_the_class() {
        let err = Error::ExceptionPending {
            class: "RuntimeException".into(),
        };

        assert_eq!(
            err.to_string(),
            "a PHP exception of class RuntimeException is pending"
        );
    }

    #[test]
    fn catch_error_is_chained_as_a_source() {
        let err: Error = CatchError::Bailout.into();

        assert_eq!(
            err.to_string(),
            "try_catch did not run the closure to completion"
        );
        assert_eq!(
            err.source().map(ToString::to_string),
            Some("the engine bailed out".to_string())
        );
    }

    #[test]
    fn invalid_cstring_reports_the_nul_position() {
        let err: Error = CString::new("a\0b")
            .expect_err("the string has a NUL")
            .into();

        assert_eq!(err.to_string(), "string contains a NUL byte at position 1");
    }

    #[test]
    fn invalid_property_names_the_property() {
        let err = Error::InvalidProperty {
            property: "age".into(),
        };

        assert_eq!(
            err.to_string(),
            "property `age` does not exist on the object"
        );
    }

    #[test]
    fn messages_are_lowercase_fragments() {
        assert_eq!(Error::InvalidScope.to_string(), "invalid scope");
    }
}
