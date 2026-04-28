//! PHP argument and return type expressions.
//!
//! [`PhpType`] is the single vocabulary used by [`Arg`](crate::args::Arg) to
//! describe every shape of PHP type declaration that ext-php-rs supports.
//! [`PhpType::Simple`], primitive [`PhpType::Union`], and class
//! [`PhpType::ClassUnion`] are handled today; later work will extend the enum
//! with intersections and DNF combinations.

use crate::flags::DataType;

/// A PHP type expression as used in argument or return position.
///
/// `Simple` covers the long-standing single-type form (`int`, `string`,
/// `Foo`, ...). `Union` covers a primitive union such as `int|string`.
/// `ClassUnion` covers a union of class names such as `Foo|Bar`.
///
/// A `Union` carrying fewer than two members is technically constructable but
/// semantically equivalent to (or weaker than) a [`PhpType::Simple`]; callers
/// should prefer `Simple` for the single-type case. The runtime does not
/// auto-collapse unions: collapsing is the parser's job in a later step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhpType {
    /// A single type, e.g. `int`, `string`, `Foo`.
    Simple(DataType),
    /// A union of primitive types, e.g. `int|string`.
    ///
    /// Including [`DataType::Null`] as a member produces a nullable union
    /// (`int|string|null`). The same shape can be expressed by combining a
    /// non-null `Union` with [`Arg::allow_null`](crate::args::Arg::allow_null);
    /// both forms emit identical bits because `MAY_BE_NULL` and
    /// `_ZEND_TYPE_NULLABLE_BIT` share the same value (see
    /// `Zend/zend_types.h:148` in php-src). Pick whichever reads best at
    /// the call site.
    Union(Vec<DataType>),
    /// A union of class names, e.g. `Foo|Bar`. Each entry must be a valid
    /// PHP class name (no NUL bytes).
    ///
    /// A single-element vec is accepted but degenerate: prefer
    /// `Simple(DataType::Object(Some(name)))` for the single-class case.
    ///
    /// Mixing primitives and classes (e.g. `int|Foo`) is not yet
    /// expressible; that is the job of the future DNF representation.
    ///
    /// Nullability flows through [`Arg::allow_null`](crate::args::Arg::allow_null);
    /// PHP's `?Foo|Bar` shorthand is not legal syntax (the engine rejects
    /// `?` on a union), so the rendered stub spells nullables as
    /// `Foo|Bar|null`.
    ClassUnion(Vec<String>),
}

impl From<DataType> for PhpType {
    fn from(dt: DataType) -> Self {
        Self::Simple(dt)
    }
}

const _: () = {
    assert!(core::mem::size_of::<PhpType>() <= 32);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_union_round_trips_through_clone_and_eq() {
        let foo_or_bar = PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()]);
        assert_eq!(foo_or_bar.clone(), foo_or_bar);
    }

    #[test]
    fn class_union_is_distinct_from_primitive_union_and_simple() {
        let class = PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()]);
        let primitive = PhpType::Union(vec![DataType::Long, DataType::String]);
        let simple = PhpType::Simple(DataType::String);

        assert_ne!(class, primitive);
        assert_ne!(class, simple);
    }
}
