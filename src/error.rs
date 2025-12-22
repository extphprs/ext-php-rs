//! Error and result types returned from the library functions.

use std::{
    error::Error as ErrorTrait,
    ffi::{CString, NulError},
    fmt::Display,
    num::TryFromIntError,
};

use crate::{
    boxed::ZBox,
    exception::PhpException,
    ffi::php_error_docref,
    flags::{ClassFlags, DataType, ErrorType, ZvalTypeFlags},
    types::ZendObject,
};

/// The main result type which is passed by the library.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The main error type which is passed by the library inside the custom
/// [`Result`] type.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An incorrect number of arguments was given to a PHP function.
    ///
    /// The enum carries two integers - the first representing the minimum
    /// number of arguments expected, and the second representing the number of
    /// arguments that were received.
    IncorrectArguments(usize, usize),
    /// There was an error converting a Zval into a primitive type.
    ///
    /// The enum carries the data type of the Zval.
    ZvalConversion(DataType),
    /// The type of the Zval is unknown.
    ///
    /// The enum carries the integer representation of the type of Zval.
    UnknownDatatype(u32),
    /// Attempted to convert a [`ZvalTypeFlags`] struct to a [`DataType`].
    /// The flags did not contain a datatype.
    ///
    /// The enum carries the flags that were attempted to be converted to a
    /// [`DataType`].
    InvalidTypeToDatatype(ZvalTypeFlags),
    /// The function called was called in an invalid scope (calling
    /// class-related functions inside of a non-class bound function).
    InvalidScope,
    /// The pointer inside a given type was invalid, either null or pointing to
    /// garbage.
    InvalidPointer,
    /// The given property name does not exist.
    InvalidProperty,
    /// The string could not be converted into a C-string due to the presence of
    /// a NUL character.
    InvalidCString,
    /// The string could not be converted into a valid Utf8 string
    InvalidUtf8,
    /// Could not call the given function.
    Callable,
    /// An object was expected.
    Object,
    /// An invalid exception type was thrown.
    InvalidException(ClassFlags),
    /// Converting integer arguments resulted in an overflow.
    IntegerOverflow,
    /// An exception was thrown in a function.
    Exception(ZBox<ZendObject>),
    /// A failure occurred while registering the stream wrapper
    StreamWrapperRegistrationFailure,
    /// A failure occurred while unregistering the stream wrapper
    StreamWrapperUnregistrationFailure,
    /// The SAPI write function is not available
    SapiWriteUnavailable,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IncorrectArguments(n, expected) => write!(
                f,
                "Expected at least {expected} arguments, got {n} arguments."
            ),
            Error::ZvalConversion(ty) => write!(
                f,
                "Could not convert Zval from type {ty} into primitive type."
            ),
            Error::UnknownDatatype(dt) => write!(f, "Unknown datatype {dt}."),
            Error::InvalidTypeToDatatype(dt) => {
                write!(f, "Type flags did not contain a datatype: {dt:?}")
            }
            Error::InvalidScope => write!(f, "Invalid scope."),
            Error::InvalidPointer => write!(f, "Invalid pointer."),
            Error::InvalidProperty => write!(f, "Property does not exist on object."),
            Error::InvalidCString => write!(
                f,
                "String given contains NUL-bytes which cannot be present in a C string."
            ),
            Error::InvalidUtf8 => write!(f, "Invalid Utf8 byte sequence."),
            Error::Callable => write!(f, "Could not call given function."),
            Error::Object => write!(f, "An object was expected."),
            Error::InvalidException(flags) => {
                write!(f, "Invalid exception type was thrown: {flags:?}")
            }
            Error::IntegerOverflow => {
                write!(f, "Converting integer arguments resulted in an overflow.")
            }
            Error::Exception(e) => write!(f, "Exception was thrown: {e:?}"),
            Error::StreamWrapperRegistrationFailure => {
                write!(f, "A failure occurred while registering the stream wrapper")
            }
            Error::StreamWrapperUnregistrationFailure => {
                write!(
                    f,
                    "A failure occurred while unregistering the stream wrapper"
                )
            }
            Error::SapiWriteUnavailable => {
                write!(f, "The SAPI write function is not available")
            }
        }
    }
}

impl ErrorTrait for Error {}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Self::InvalidCString
    }
}

impl From<TryFromIntError> for Error {
    fn from(_value: TryFromIntError) -> Self {
        Self::IntegerOverflow
    }
}

impl From<Error> for PhpException {
    fn from(err: Error) -> Self {
        Self::default(err.to_string())
    }
}

/// Trigger an error that is reported in PHP the same way `trigger_error()` is.
///
/// See specific error type descriptions at <https://www.php.net/manual/en/errorfunc.constants.php>.
///
/// # Panics
///
/// * If the error type bits exceed `i32::MAX`.
pub fn php_error(type_: &ErrorType, message: &str) {
    let Ok(c_string) = CString::new(message) else {
        return;
    };

    unsafe {
        php_error_docref(
            std::ptr::null(),
            type_.bits().try_into().expect("Error type flags overflown"),
            c_string.as_ptr(),
        );
    }
}
