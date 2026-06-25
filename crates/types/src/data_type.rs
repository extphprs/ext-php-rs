//! [`DataType`] — the value-side enum the parser produces for primitive type
//! names. Lives here, not in the runtime crate, because [`crate::PhpType`]
//! carries it and the parser must construct it.
//!
//! The runtime crate hangs FFI conversion helpers off this enum (e.g.
//! `ext_php_rs::flags::data_type_from_raw`); those depend on PHP's
//! `IS_*` constants and stay there.

use std::fmt::{self, Display};

/// Valid data types for a [`Zval`](https://docs.rs/ext-php-rs/latest/ext_php_rs/types/struct.Zval.html).
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[cfg(feature = "proc-macro")]
impl quote::ToTokens for DataType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use quote::quote;
        let stream = match self {
            DataType::Undef => quote!(::ext_php_rs::flags::DataType::Undef),
            DataType::Null => quote!(::ext_php_rs::flags::DataType::Null),
            DataType::False => quote!(::ext_php_rs::flags::DataType::False),
            DataType::True => quote!(::ext_php_rs::flags::DataType::True),
            DataType::Long => quote!(::ext_php_rs::flags::DataType::Long),
            DataType::Double => quote!(::ext_php_rs::flags::DataType::Double),
            DataType::String => quote!(::ext_php_rs::flags::DataType::String),
            DataType::Array => quote!(::ext_php_rs::flags::DataType::Array),
            DataType::Iterable => quote!(::ext_php_rs::flags::DataType::Iterable),
            DataType::Object(None) => {
                quote!(::ext_php_rs::flags::DataType::Object(
                    ::core::option::Option::None
                ))
            }
            DataType::Object(Some(name)) => {
                quote!(
                    ::ext_php_rs::flags::DataType::Object(
                        ::core::option::Option::Some(#name)
                    )
                )
            }
            DataType::Resource => quote!(::ext_php_rs::flags::DataType::Resource),
            DataType::Reference => quote!(::ext_php_rs::flags::DataType::Reference),
            DataType::Callable => quote!(::ext_php_rs::flags::DataType::Callable),
            DataType::ConstantExpression => {
                quote!(::ext_php_rs::flags::DataType::ConstantExpression)
            }
            DataType::Void => quote!(::ext_php_rs::flags::DataType::Void),
            DataType::Mixed => quote!(::ext_php_rs::flags::DataType::Mixed),
            DataType::Bool => quote!(::ext_php_rs::flags::DataType::Bool),
            DataType::Ptr => quote!(::ext_php_rs::flags::DataType::Ptr),
            DataType::Indirect => quote!(::ext_php_rs::flags::DataType::Indirect),
        };
        stream.to_tokens(tokens);
    }
}

#[cfg(all(test, feature = "proc-macro"))]
mod tokens_tests {
    use super::DataType;
    use quote::quote;

    fn render<T: quote::ToTokens>(value: &T) -> String {
        quote!(#value).to_string()
    }

    #[test]
    fn each_primitive_emits_the_runtime_path() {
        let cases: &[(DataType, proc_macro2::TokenStream)] = &[
            (DataType::Long, quote!(::ext_php_rs::flags::DataType::Long)),
            (DataType::Null, quote!(::ext_php_rs::flags::DataType::Null)),
            (
                DataType::String,
                quote!(::ext_php_rs::flags::DataType::String),
            ),
            (DataType::Bool, quote!(::ext_php_rs::flags::DataType::Bool)),
            (DataType::True, quote!(::ext_php_rs::flags::DataType::True)),
            (
                DataType::Double,
                quote!(::ext_php_rs::flags::DataType::Double),
            ),
            (DataType::Void, quote!(::ext_php_rs::flags::DataType::Void)),
            (
                DataType::Mixed,
                quote!(::ext_php_rs::flags::DataType::Mixed),
            ),
            (
                DataType::Iterable,
                quote!(::ext_php_rs::flags::DataType::Iterable),
            ),
            (
                DataType::Callable,
                quote!(::ext_php_rs::flags::DataType::Callable),
            ),
        ];
        for (dt, expected) in cases {
            assert_eq!(render(dt), expected.to_string(), "DataType::{dt:?}");
        }
    }

    #[test]
    fn object_none_emits_explicit_option_none() {
        let dt = DataType::Object(None);
        assert_eq!(
            render(&dt),
            quote!(::ext_php_rs::flags::DataType::Object(
                ::core::option::Option::None
            ))
            .to_string()
        );
    }

    #[test]
    fn object_some_inlines_static_name() {
        let dt = DataType::Object(Some("Foo"));
        assert_eq!(
            render(&dt),
            quote!(::ext_php_rs::flags::DataType::Object(
                ::core::option::Option::Some("Foo")
            ))
            .to_string()
        );
    }
}
