//! PHP argument and return type expressions.
//!
//! [`PhpType`] is the single vocabulary used by [`Arg`](crate::args::Arg) to
//! describe every shape of PHP type declaration that ext-php-rs supports:
//! [`PhpType::Simple`], primitive [`PhpType::Union`], class
//! [`PhpType::ClassUnion`], class [`PhpType::Intersection`] (PHP 8.1+), and
//! the disjunctive normal form [`PhpType::Dnf`] (PHP 8.2+).

use crate::flags::DataType;

/// One disjunct of a [`PhpType::Dnf`] type. PHP 8.2+.
///
/// PHP's DNF grammar is a top-level union whose alternatives may themselves
/// be intersection groups, e.g. `(A&B)|C`. Each [`DnfTerm`] is one alternative
/// on the union side: either a single class name (the `C`) or an intersection
/// group (the `A&B`).
///
/// `Intersection` always carries 2 or more members. A single-element group is
/// rejected by the FFI emission layer; callers should use [`DnfTerm::Single`]
/// for one-class disjuncts. The future type-string parser canonicalises this
/// shape automatically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnfTerm {
    /// A single class name, e.g. the `C` in `(A&B)|C`. Class names must be
    /// non-empty and contain no interior NUL bytes.
    Single(String),
    /// An intersection group of class/interface names, e.g. the `A&B` in
    /// `(A&B)|C`. Always carries 2 or more members; one-element groups are
    /// rejected at the FFI emission layer (use [`DnfTerm::Single`] instead).
    Intersection(Vec<String>),
}

/// A PHP type expression as used in argument or return position.
///
/// `Simple` covers the long-standing single-type form (`int`, `string`,
/// `Foo`, ...). `Union` covers a primitive union such as `int|string`.
/// `ClassUnion` covers a union of class names such as `Foo|Bar`.
/// `Intersection` covers `Countable&Traversable`. `Dnf` covers
/// `(A&B)|C` and its nullable form `(A&B)|null`.
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
    /// Mixing primitives and classes (e.g. `int|Foo`) is not expressible
    /// here; class-side DNF such as `(A&B)|C` lives in [`PhpType::Dnf`].
    ///
    /// Nullability flows through [`Arg::allow_null`](crate::args::Arg::allow_null);
    /// PHP's `?Foo|Bar` shorthand is not legal syntax (the engine rejects
    /// `?` on a union), so the rendered stub spells nullables as
    /// `Foo|Bar|null`.
    ClassUnion(Vec<String>),
    /// An intersection of class/interface names, e.g. `Countable&Traversable`.
    /// A value satisfies the type only when it is an instance of every named
    /// class or interface. Each entry must be a valid PHP class name (no NUL
    /// bytes).
    ///
    /// A single-element vec is accepted but degenerate: prefer
    /// `Simple(DataType::Object(Some(name)))` for the single-class case.
    ///
    /// Pairing this variant with
    /// [`Arg::allow_null`](crate::args::Arg::allow_null) is rejected by the
    /// FFI emission layer. The legal nullable form is the DNF
    /// `(Foo&Bar)|null`; build a [`PhpType::Dnf`] for that case.
    Intersection(Vec<String>),
    /// Disjunctive Normal Form: a top-level union whose alternatives may
    /// themselves be intersection groups, e.g. `(A&B)|C`. PHP 8.2+.
    ///
    /// Examples:
    /// - `(A&B)|C` produces
    ///   `Dnf(vec![DnfTerm::Intersection(["A","B"]), DnfTerm::Single("C")])`.
    /// - `(A&B)|null` produces
    ///   `Dnf(vec![DnfTerm::Intersection(["A","B"])])` with
    ///   [`Arg::allow_null`](crate::args::Arg::allow_null) on the arg.
    ///
    /// Nullability is carried via `allow_null`, never as a stringly-typed
    /// `DnfTerm::Single("null")` term — the same canonicalisation rule the
    /// other compound variants follow. Mixing primitives with class terms
    /// (e.g. `(A&B)|int`) is intentionally not modelled here; if demand
    /// surfaces, [`DnfTerm`] can grow a third variant in a follow-up.
    ///
    /// Validation (see the FFI emission layer): empty `terms` is rejected;
    /// `terms.len() == 1` is degenerate (use [`PhpType::Simple`] or
    /// [`PhpType::Intersection`]); each
    /// [`DnfTerm::Intersection`] must carry 2 or more members.
    Dnf(Vec<DnfTerm>),
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

    #[test]
    fn intersection_round_trips_through_clone_and_eq() {
        let countable_and_traversable =
            PhpType::Intersection(vec!["Countable".to_owned(), "Traversable".to_owned()]);
        assert_eq!(countable_and_traversable.clone(), countable_and_traversable);
    }

    #[test]
    fn intersection_is_distinct_from_class_union_simple_and_primitive_union() {
        let intersection = PhpType::Intersection(vec!["Foo".to_owned(), "Bar".to_owned()]);
        let class_union = PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()]);
        let primitive = PhpType::Union(vec![DataType::Long, DataType::String]);
        let simple = PhpType::Simple(DataType::String);

        assert_ne!(intersection, class_union);
        assert_ne!(intersection, primitive);
        assert_ne!(intersection, simple);
    }

    #[test]
    fn dnf_round_trips_through_clone_and_eq() {
        let dnf = PhpType::Dnf(vec![
            DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
            DnfTerm::Single("C".to_owned()),
        ]);
        assert_eq!(dnf.clone(), dnf);
    }

    #[test]
    fn dnf_is_distinct_from_intersection_class_union_and_simple() {
        let dnf = PhpType::Dnf(vec![
            DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
            DnfTerm::Single("C".to_owned()),
        ]);
        let intersection = PhpType::Intersection(vec!["A".to_owned(), "B".to_owned()]);
        let class_union = PhpType::ClassUnion(vec!["A".to_owned(), "C".to_owned()]);
        let simple = PhpType::Simple(DataType::String);

        assert_ne!(dnf, intersection);
        assert_ne!(dnf, class_union);
        assert_ne!(dnf, simple);
    }

    #[test]
    fn dnf_term_round_trips_through_clone_and_eq() {
        let single = DnfTerm::Single("Foo".to_owned());
        let group = DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]);
        assert_eq!(single.clone(), single);
        assert_eq!(group.clone(), group);
        assert_ne!(single, group);
    }
}
