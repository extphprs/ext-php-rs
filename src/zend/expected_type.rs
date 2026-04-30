//! Safe wrapper around PHP's legacy `_zend_expected_type` discriminant.
//!
//! [`ExpectedType`] is a typed Rust mirror of the small set of `Z_EXPECTED_*`
//! values that ext-php-rs supports for the legacy ZPP error path. Callers
//! receive an [`ExpectedType`] from [`crate::args::Arg::expected_type`] and
//! pass it to [`wrong_parameter_type_error`] without ever touching the raw
//! FFI integer.

use crate::ffi;
use crate::flags::DataType;
use crate::types::Zval;

/// Subset of PHP's `Z_EXPECTED_*` discriminants that map cleanly to a single
/// scalar [`crate::flags::DataType`].
///
/// Compound PHP types (unions, intersections, DNF) have no equivalent in
/// PHP's fixed enum and are reported through the modern
/// `zend_argument_type_error` path instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExpectedType {
    /// `int` — `Z_EXPECTED_LONG`.
    Long,
    /// `?int` — `Z_EXPECTED_LONG_OR_NULL`.
    LongOrNull,
    /// `bool` — `Z_EXPECTED_BOOL`.
    Bool,
    /// `?bool` — `Z_EXPECTED_BOOL_OR_NULL`.
    BoolOrNull,
    /// `string` — `Z_EXPECTED_STRING`.
    String,
    /// `?string` — `Z_EXPECTED_STRING_OR_NULL`.
    StringOrNull,
    /// `array` — `Z_EXPECTED_ARRAY`.
    Array,
    /// `?array` — `Z_EXPECTED_ARRAY_OR_NULL`.
    ArrayOrNull,
    /// `object` — `Z_EXPECTED_OBJECT`.
    Object,
    /// `?object` — `Z_EXPECTED_OBJECT_OR_NULL`.
    ObjectOrNull,
    /// `float` — `Z_EXPECTED_DOUBLE`.
    Double,
    /// `?float` — `Z_EXPECTED_DOUBLE_OR_NULL`.
    DoubleOrNull,
    /// `resource` — `Z_EXPECTED_RESOURCE`.
    Resource,
    /// `?resource` — `Z_EXPECTED_RESOURCE_OR_NULL`.
    ResourceOrNull,
}

impl ExpectedType {
    /// Map a scalar [`DataType`] plus a nullability flag to the matching
    /// discriminant. Returns `None` for `DataType` variants that have no
    /// `Z_EXPECTED_*` slot (e.g. `Mixed`, `Void`, `Iterable`, `Callable`,
    /// `Null`).
    pub(crate) fn from_simple(dt: DataType, nullable: bool) -> Option<Self> {
        Some(match (dt, nullable) {
            (DataType::Long, false) => Self::Long,
            (DataType::Long, true) => Self::LongOrNull,
            (DataType::Bool | DataType::True | DataType::False, false) => Self::Bool,
            (DataType::Bool | DataType::True | DataType::False, true) => Self::BoolOrNull,
            (DataType::String, false) => Self::String,
            (DataType::String, true) => Self::StringOrNull,
            (DataType::Array, false) => Self::Array,
            (DataType::Array, true) => Self::ArrayOrNull,
            (DataType::Object(_), false) => Self::Object,
            (DataType::Object(_), true) => Self::ObjectOrNull,
            (DataType::Double, false) => Self::Double,
            (DataType::Double, true) => Self::DoubleOrNull,
            (DataType::Resource, false) => Self::Resource,
            (DataType::Resource, true) => Self::ResourceOrNull,
            _ => return None,
        })
    }

    pub(crate) fn into_raw(self) -> ffi::_zend_expected_type {
        match self {
            Self::Long => ffi::_zend_expected_type_Z_EXPECTED_LONG,
            Self::LongOrNull => ffi::_zend_expected_type_Z_EXPECTED_LONG_OR_NULL,
            Self::Bool => ffi::_zend_expected_type_Z_EXPECTED_BOOL,
            Self::BoolOrNull => ffi::_zend_expected_type_Z_EXPECTED_BOOL_OR_NULL,
            Self::String => ffi::_zend_expected_type_Z_EXPECTED_STRING,
            Self::StringOrNull => ffi::_zend_expected_type_Z_EXPECTED_STRING_OR_NULL,
            Self::Array => ffi::_zend_expected_type_Z_EXPECTED_ARRAY,
            Self::ArrayOrNull => ffi::_zend_expected_type_Z_EXPECTED_ARRAY_OR_NULL,
            Self::Object => ffi::_zend_expected_type_Z_EXPECTED_OBJECT,
            Self::ObjectOrNull => ffi::_zend_expected_type_Z_EXPECTED_OBJECT_OR_NULL,
            Self::Double => ffi::_zend_expected_type_Z_EXPECTED_DOUBLE,
            Self::DoubleOrNull => ffi::_zend_expected_type_Z_EXPECTED_DOUBLE_OR_NULL,
            Self::Resource => ffi::_zend_expected_type_Z_EXPECTED_RESOURCE,
            Self::ResourceOrNull => ffi::_zend_expected_type_Z_EXPECTED_RESOURCE_OR_NULL,
        }
    }
}

/// Reports a wrong-type argument through PHP's legacy ZPP error helper
/// (`zend_wrong_parameter_type_error`).
///
/// Use this when you have a scalar [`ExpectedType`] from
/// [`crate::args::Arg::expected_type`]. For compound declared types use
/// [`crate::exception::PhpException`] or call `zend_argument_type_error`
/// with a custom message built from [`crate::args::Arg::ty`].
///
/// # Parameters
///
/// * `arg_num` - 1-based argument index, as PHP expects.
/// * `expected` - The expected type discriminant.
/// * `given` - The actual value PHP received.
pub fn wrong_parameter_type_error(arg_num: u32, expected: ExpectedType, given: &Zval) {
    // SAFETY: `given` is a live `&Zval`. PHP's C signature is `const zval *`,
    // but bindgen drops `const` on pointer parameters; the cast to `*mut`
    // is sound because the engine only reads the value.
    unsafe {
        ffi::zend_wrong_parameter_type_error(
            arg_num,
            expected.into_raw(),
            std::ptr::from_ref::<Zval>(given).cast_mut(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi;
    use crate::flags::DataType;

    #[test]
    fn wrong_parameter_type_error_signature_is_stable() {
        // Catches FFI-binding drift if PHP renames or re-shapes
        // `zend_wrong_parameter_type_error`. Behavioural verification (the
        // engine actually queues a TypeError) requires an active execute
        // frame and lives in the integration test suite, not here, because
        // PHP's helper calls `get_active_function_or_method_name()` which
        // asserts `zend_is_executing()`.
        let _: fn(u32, ExpectedType, &Zval) = wrong_parameter_type_error;
    }

    #[test]
    fn from_simple_long() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Long, false),
            Some(ExpectedType::Long),
        );
    }

    #[test]
    fn from_simple_long_nullable() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Long, true),
            Some(ExpectedType::LongOrNull),
        );
    }

    #[test]
    fn from_simple_bool_via_false_alias() {
        assert_eq!(
            ExpectedType::from_simple(DataType::False, false),
            Some(ExpectedType::Bool),
        );
    }

    #[test]
    fn from_simple_bool_via_true_alias() {
        assert_eq!(
            ExpectedType::from_simple(DataType::True, false),
            Some(ExpectedType::Bool),
        );
    }

    #[test]
    fn from_simple_bool_nullable() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Bool, true),
            Some(ExpectedType::BoolOrNull),
        );
    }

    #[test]
    fn from_simple_string() {
        assert_eq!(
            ExpectedType::from_simple(DataType::String, false),
            Some(ExpectedType::String),
        );
    }

    #[test]
    fn from_simple_string_nullable() {
        assert_eq!(
            ExpectedType::from_simple(DataType::String, true),
            Some(ExpectedType::StringOrNull),
        );
    }

    #[test]
    fn from_simple_array() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Array, false),
            Some(ExpectedType::Array),
        );
    }

    #[test]
    fn from_simple_array_nullable() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Array, true),
            Some(ExpectedType::ArrayOrNull),
        );
    }

    #[test]
    fn from_simple_object_with_class_name() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Object(Some("Foo")), false),
            Some(ExpectedType::Object),
        );
    }

    #[test]
    fn from_simple_object_without_class_name_nullable() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Object(None), true),
            Some(ExpectedType::ObjectOrNull),
        );
    }

    #[test]
    fn from_simple_double() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Double, false),
            Some(ExpectedType::Double),
        );
    }

    #[test]
    fn from_simple_double_nullable() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Double, true),
            Some(ExpectedType::DoubleOrNull),
        );
    }

    #[test]
    fn from_simple_resource() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Resource, false),
            Some(ExpectedType::Resource),
        );
    }

    #[test]
    fn from_simple_resource_nullable() {
        assert_eq!(
            ExpectedType::from_simple(DataType::Resource, true),
            Some(ExpectedType::ResourceOrNull),
        );
    }

    #[test]
    fn from_simple_unsupported_returns_none() {
        assert!(ExpectedType::from_simple(DataType::Mixed, false).is_none());
        assert!(ExpectedType::from_simple(DataType::Void, false).is_none());
        assert!(ExpectedType::from_simple(DataType::Iterable, false).is_none());
        assert!(ExpectedType::from_simple(DataType::Callable, false).is_none());
        assert!(ExpectedType::from_simple(DataType::Null, false).is_none());
    }

    #[test]
    fn into_raw_long() {
        assert_eq!(
            ExpectedType::Long.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_LONG
        );
    }

    #[test]
    fn into_raw_long_or_null() {
        assert_eq!(
            ExpectedType::LongOrNull.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_LONG_OR_NULL
        );
    }

    #[test]
    fn into_raw_bool() {
        assert_eq!(
            ExpectedType::Bool.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_BOOL
        );
    }

    #[test]
    fn into_raw_bool_or_null() {
        assert_eq!(
            ExpectedType::BoolOrNull.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_BOOL_OR_NULL
        );
    }

    #[test]
    fn into_raw_string() {
        assert_eq!(
            ExpectedType::String.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_STRING
        );
    }

    #[test]
    fn into_raw_string_or_null() {
        assert_eq!(
            ExpectedType::StringOrNull.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_STRING_OR_NULL
        );
    }

    #[test]
    fn into_raw_array() {
        assert_eq!(
            ExpectedType::Array.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_ARRAY
        );
    }

    #[test]
    fn into_raw_array_or_null() {
        assert_eq!(
            ExpectedType::ArrayOrNull.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_ARRAY_OR_NULL
        );
    }

    #[test]
    fn into_raw_object() {
        assert_eq!(
            ExpectedType::Object.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_OBJECT
        );
    }

    #[test]
    fn into_raw_object_or_null() {
        assert_eq!(
            ExpectedType::ObjectOrNull.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_OBJECT_OR_NULL
        );
    }

    #[test]
    fn into_raw_double() {
        assert_eq!(
            ExpectedType::Double.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_DOUBLE
        );
    }

    #[test]
    fn into_raw_double_or_null() {
        assert_eq!(
            ExpectedType::DoubleOrNull.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_DOUBLE_OR_NULL
        );
    }

    #[test]
    fn into_raw_resource() {
        assert_eq!(
            ExpectedType::Resource.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_RESOURCE
        );
    }

    #[test]
    fn into_raw_resource_or_null() {
        assert_eq!(
            ExpectedType::ResourceOrNull.into_raw(),
            ffi::_zend_expected_type_Z_EXPECTED_RESOURCE_OR_NULL
        );
    }
}
