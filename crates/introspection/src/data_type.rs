use std::fmt::Display;

/// Valid data types for PHP.
///
/// The Zend integer representation of each variant depends on the PHP version
/// and lives in `ext-php-rs`, behind bindgen. This crate only carries the
/// ABI-stable enum so `cargo-php` can read it from a compiled extension.
#[repr(C, u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum DataType {
    /// Undefined
    Undef,
    /// `null`
    Null,
    /// `false`
    False,
    /// `true`
    True,
    /// Integer (the irony)
    Long,
    /// Floating point number
    Double,
    /// String
    String,
    /// Array
    Array,
    /// Iterable
    Iterable,
    /// Object
    Object(Option<&'static str>),
    /// Resource
    Resource,
    /// Reference
    Reference,
    /// Callable
    Callable,
    /// Constant expression
    ConstantExpression,
    /// Void
    #[default]
    Void,
    /// Mixed
    Mixed,
    /// Boolean
    Bool,
    /// Pointer
    Ptr,
    /// Indirect (internal)
    Indirect,
}

impl Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Undef => write!(f, "Undefined"),
            DataType::Null => write!(f, "Null"),
            DataType::False => write!(f, "False"),
            DataType::True => write!(f, "True"),
            DataType::Long => write!(f, "Long"),
            DataType::Double => write!(f, "Double"),
            DataType::String => write!(f, "String"),
            DataType::Array => write!(f, "Array"),
            DataType::Object(obj) => write!(f, "{}", obj.as_deref().unwrap_or("Object")),
            DataType::Resource => write!(f, "Resource"),
            DataType::Reference => write!(f, "Reference"),
            DataType::Callable => write!(f, "Callable"),
            DataType::ConstantExpression => write!(f, "Constant Expression"),
            DataType::Void => write!(f, "Void"),
            DataType::Bool => write!(f, "Bool"),
            DataType::Mixed => write!(f, "Mixed"),
            DataType::Ptr => write!(f, "Pointer"),
            DataType::Indirect => write!(f, "Indirect"),
            DataType::Iterable => write!(f, "Iterable"),
        }
    }
}
