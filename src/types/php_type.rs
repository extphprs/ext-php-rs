//! PHP argument and return type expressions.
//!
//! [`PhpType`] is the single vocabulary used by [`Arg`](crate::args::Arg) to
//! describe every shape of PHP type declaration that ext-php-rs supports.
//! Only the [`PhpType::Simple`] and primitive [`PhpType::Union`] forms are
//! handled today; later work will extend the enum with class unions,
//! intersections, and DNF combinations.

use crate::flags::DataType;

/// A PHP type expression as used in argument or return position.
///
/// `Simple` covers the long-standing single-type form (`int`, `string`,
/// `Foo`, ...). `Union` covers a primitive union such as `int|string`.
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
    Union(Vec<DataType>),
}

impl From<DataType> for PhpType {
    fn from(dt: DataType) -> Self {
        Self::Simple(dt)
    }
}

const _: () = {
    assert!(core::mem::size_of::<PhpType>() <= 32);
};
