//! PHP argument and return type expressions.
//!
//! [`PhpType`] is the single vocabulary used by [`Arg`](crate::args::Arg) to
//! describe every shape of PHP type declaration that ext-php-rs supports:
//! [`PhpType::Simple`], primitive [`PhpType::Union`], class
//! [`PhpType::ClassUnion`], class [`PhpType::Intersection`] (PHP 8.1+), and
//! the disjunctive normal form [`PhpType::Dnf`] (PHP 8.2+).

use std::fmt;
use std::str::FromStr;

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

impl fmt::Display for PhpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simple(dt) => write_php_primitive_or_class(*dt, f),
            Self::Union(members) => {
                let mut first = true;
                for dt in members {
                    if !first {
                        f.write_str("|")?;
                    }
                    write_php_primitive_or_class(*dt, f)?;
                    first = false;
                }
                Ok(())
            }
            Self::ClassUnion(names) => write_pipe_joined_classes(names, f),
            Self::Intersection(names) => write_amp_joined_classes(names, f),
            Self::Dnf(terms) => {
                let mut first = true;
                for term in terms {
                    if !first {
                        f.write_str("|")?;
                    }
                    fmt::Display::fmt(term, f)?;
                    first = false;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for DnfTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(name) => write_class_name(name, f),
            Self::Intersection(names) => {
                f.write_str("(")?;
                write_amp_joined_classes(names, f)?;
                f.write_str(")")
            }
        }
    }
}

fn write_php_primitive_or_class(dt: DataType, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match dt {
        DataType::Bool => f.write_str("bool"),
        DataType::True => f.write_str("true"),
        DataType::False => f.write_str("false"),
        DataType::Long => f.write_str("int"),
        DataType::Double => f.write_str("float"),
        DataType::String => f.write_str("string"),
        DataType::Array => f.write_str("array"),
        DataType::Object(Some(name)) => write_class_name(name, f),
        DataType::Object(None) => f.write_str("object"),
        DataType::Resource => f.write_str("resource"),
        DataType::Callable => f.write_str("callable"),
        DataType::Iterable => f.write_str("iterable"),
        DataType::Void => f.write_str("void"),
        DataType::Null => f.write_str("null"),
        // `Mixed` plus the variants without a syntactic PHP type form
        // (`Undef`, `Reference`, `ConstantExpression`, `Ptr`, `Indirect`)
        // all render as `mixed`, matching `datatype_to_phpdoc` in
        // `src/describe/stub.rs`.
        DataType::Mixed
        | DataType::Undef
        | DataType::Reference
        | DataType::ConstantExpression
        | DataType::Ptr
        | DataType::Indirect => f.write_str("mixed"),
    }
}

fn write_class_name(name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if name.starts_with('\\') {
        f.write_str(name)
    } else {
        f.write_str("\\")?;
        f.write_str(name)
    }
}

fn write_pipe_joined_classes(names: &[String], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut first = true;
    for name in names {
        if !first {
            f.write_str("|")?;
        }
        write_class_name(name, f)?;
        first = false;
    }
    Ok(())
}

fn write_amp_joined_classes(names: &[String], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut first = true;
    for name in names {
        if !first {
            f.write_str("&")?;
        }
        write_class_name(name, f)?;
        first = false;
    }
    Ok(())
}

const _: () = {
    assert!(core::mem::size_of::<PhpType>() <= 32);
};

/// Error produced by [`PhpType::from_str`].
///
/// The parser surfaces every failure mode that the runtime crate can check
/// without round-tripping through `zend_compile.c`. Variants carry byte
/// positions in the input where useful so callers (especially the future
/// `#[php(types = "...")]` proc-macro) can underline the offending span.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PhpTypeParseError {
    /// Input was empty or whitespace-only.
    Empty,
    /// A `|`-separated alternative was empty (e.g. leading or trailing pipe,
    /// or two pipes in a row).
    EmptyTerm { pos: usize },
    /// A `(` was opened without a matching `)`, or vice versa.
    UnbalancedParens { pos: usize },
    /// An unexpected character was encountered (control byte, stray comma,
    /// nested `(`, etc.).
    UnexpectedChar { ch: char, pos: usize },
    /// A `(` appeared inside another `(` group: DNF only allows one level.
    NestedGroups { pos: usize },
    /// A `|` appeared inside an intersection group: PHP rejects unions
    /// nested inside intersections (`A&(B|C)` is illegal).
    UnionInIntersection { pos: usize },
    /// A bare `&` appeared outside a `( ... )` group at union level: PHP's
    /// grammar refuses `A&B|C` because `intersection_type` is not a
    /// `union_type_element` without parens.
    NakedAmpInUnion { pos: usize },
    /// A `?` shorthand was applied to a compound type (`?int|string`,
    /// `?A&B`, `?(A&B)`). `?` is only legal on a single primitive or class.
    NullableCompound { pos: usize },
    /// A `( ... )` group held fewer than two members. PHP requires at least
    /// `(A&B)` inside parens; `(A)` is a grammar error.
    IntersectionTooSmall { pos: usize },
    /// A class name was empty or contained an interior NUL byte (the runtime
    /// would later turn that into `Error::InvalidCString`; the parser catches
    /// it earlier).
    InvalidClassName { name: String },
    /// A keyword `static`, `never`, `self`, or `parent` appeared. ext-php-rs
    /// cannot register internal arg-info for these — they're context types
    /// the engine resolves at the call site.
    UnsupportedKeyword { name: String },
    /// The same primitive or class name appeared twice in a union or
    /// intersection. PHP rejects duplicates with
    /// "Duplicate type %s is redundant".
    DuplicateMember { name: String },
    /// A union mixed primitive types with class names (`int|Foo`). The
    /// runtime [`PhpType`] variants do not model this mixing — see the
    /// note on [`PhpType::Dnf`].
    MixedPrimitiveAndClass,
    /// The input describes a class-side type combined with `null`
    /// (`?Foo`, `Foo|null`, `Foo|Bar|null`, `(A&B)|null`). The runtime
    /// [`PhpType`] does not carry nullability for class-side variants;
    /// callers should parse the non-null form and chain
    /// [`Arg::allow_null`](crate::args::Arg::allow_null) on the resulting
    /// [`Arg`](crate::args::Arg).
    ClassNullableNotRepresentable,
    /// A primitive name appeared inside an intersection. PHP rejects
    /// `int&string` and similar shapes at compile time.
    PrimitiveInIntersection { name: String },
    /// A primitive name appeared inside a class-only context (multi-class
    /// union or DNF group). The variants `ClassUnion`/`Dnf` only carry
    /// class names; mixing primitives is rejected at construction.
    PrimitiveInClassUnion { name: String },
}

impl fmt::Display for PhpTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty type string"),
            Self::EmptyTerm { pos } => write!(f, "empty term at position {pos}"),
            Self::UnbalancedParens { pos } => {
                write!(f, "unbalanced parenthesis at position {pos}")
            }
            Self::UnexpectedChar { ch, pos } => {
                write!(f, "unexpected character {ch:?} at position {pos}")
            }
            Self::NestedGroups { pos } => {
                write!(f, "nested `(` groups not allowed at position {pos}")
            }
            Self::UnionInIntersection { pos } => write!(
                f,
                "union inside intersection at position {pos}: intersections cannot contain unions"
            ),
            Self::NakedAmpInUnion { pos } => write!(
                f,
                "bare `&` at union level (position {pos}): use parentheses, e.g. `(A&B)|C`"
            ),
            Self::NullableCompound { pos } => write!(
                f,
                "`?` shorthand at position {pos} can only apply to a single type"
            ),
            Self::IntersectionTooSmall { pos } => write!(
                f,
                "intersection group at position {pos} must contain at least two class names"
            ),
            Self::InvalidClassName { name } => {
                write!(f, "invalid class name {name:?} (empty or contains NUL)")
            }
            Self::UnsupportedKeyword { name } => write!(
                f,
                "keyword {name:?} is not supported in ext-php-rs argument and return types"
            ),
            Self::DuplicateMember { name } => write!(f, "duplicate type {name:?}"),
            Self::MixedPrimitiveAndClass => write!(
                f,
                "primitive types and class names cannot be mixed in a union"
            ),
            Self::ClassNullableNotRepresentable => write!(
                f,
                "class-side nullable type cannot be represented as a single PhpType; \
                 parse the non-null form and chain `Arg::allow_null()` on the resulting Arg"
            ),
            Self::PrimitiveInIntersection { name } => {
                write!(f, "primitive {name:?} cannot appear in an intersection")
            }
            Self::PrimitiveInClassUnion { name } => write!(
                f,
                "primitive {name:?} cannot appear in a class-only union or DNF term"
            ),
        }
    }
}

impl std::error::Error for PhpTypeParseError {}

impl FromStr for PhpType {
    type Err = PhpTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s)
    }
}

fn parse(s: &str) -> Result<PhpType, PhpTypeParseError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(PhpTypeParseError::Empty);
    }

    validate_balanced_parens(s)?;

    let (nullable, body, body_offset) = strip_nullable_prefix(s, trimmed);

    if has_top_level_char(body, '|') {
        if nullable {
            return Err(PhpTypeParseError::NullableCompound { pos: 0 });
        }
        return parse_union(body, body_offset);
    }

    if has_top_level_char(body, '&') {
        if nullable {
            return Err(PhpTypeParseError::NullableCompound { pos: 0 });
        }
        return parse_bare_intersection(body, body_offset);
    }

    if body.starts_with('(') {
        return Err(PhpTypeParseError::IntersectionTooSmall { pos: body_offset });
    }

    let single = parse_atom(body)?;
    match single {
        Atom::Primitive(dt) if nullable => Ok(PhpType::Union(vec![dt, DataType::Null])),
        Atom::Primitive(dt) => Ok(PhpType::Simple(dt)),
        Atom::Class(_) if nullable => Err(PhpTypeParseError::ClassNullableNotRepresentable),
        Atom::Class(name) => Ok(PhpType::ClassUnion(vec![name])),
    }
}

fn validate_balanced_parens(s: &str) -> Result<(), PhpTypeParseError> {
    let mut depth: usize = 0;
    let mut last_open: Option<usize> = None;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => {
                depth += 1;
                last_open = Some(i);
            }
            ')' => {
                if depth == 0 {
                    return Err(PhpTypeParseError::UnbalancedParens { pos: i });
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(PhpTypeParseError::UnbalancedParens {
            pos: last_open.unwrap_or(0),
        });
    }
    Ok(())
}

fn has_top_level_char(body: &str, target: char) -> bool {
    let mut depth = 0usize;
    for ch in body.chars() {
        match ch {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            c if c == target && depth == 0 => return true,
            _ => {}
        }
    }
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Atom {
    Primitive(DataType),
    Class(String),
}

fn strip_nullable_prefix<'a>(original: &'a str, trimmed: &'a str) -> (bool, &'a str, usize) {
    let leading_ws = original.len() - original.trim_start().len();
    if let Some(rest) = trimmed.strip_prefix('?') {
        (true, rest.trim_start(), leading_ws + 1)
    } else {
        (false, trimmed, leading_ws)
    }
}

fn parse_atom(raw: &str) -> Result<Atom, PhpTypeParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PhpTypeParseError::EmptyTerm { pos: 0 });
    }
    reject_structural_chars(trimmed)?;
    reject_unsupported_keyword(trimmed)?;
    if let Some(dt) = primitive_from_name(trimmed) {
        return Ok(Atom::Primitive(dt));
    }
    let class = normalise_class_name(trimmed)?;
    Ok(Atom::Class(class))
}

fn reject_structural_chars(name: &str) -> Result<(), PhpTypeParseError> {
    for (i, ch) in name.char_indices() {
        match ch {
            '(' | ')' | '|' | '&' | '?' | ' ' | '\t' | '\n' | '\r' => {
                return Err(PhpTypeParseError::UnexpectedChar { ch, pos: i });
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_union(body: &str, body_offset: usize) -> Result<PhpType, PhpTypeParseError> {
    let mut alts: Vec<(Alt, usize)> = Vec::new();
    for piece in split_top_level_pipes(body) {
        let span_start = body_offset + piece.start;
        let raw = &body[piece.start..piece.end];
        if raw.trim().is_empty() {
            return Err(PhpTypeParseError::EmptyTerm { pos: span_start });
        }
        alts.push((parse_alt(raw, span_start)?, span_start));
    }

    let has_group = alts.iter().any(|(a, _)| matches!(a, Alt::Group(_)));
    let has_class = alts
        .iter()
        .any(|(a, _)| matches!(a, Alt::Atom(Atom::Class(_)) | Alt::Group(_)));
    let has_null = alts
        .iter()
        .any(|(a, _)| matches!(a, Alt::Atom(Atom::Primitive(DataType::Null))));
    let has_non_null_primitive = alts.iter().any(|(a, _)| {
        matches!(
            a,
            Alt::Atom(Atom::Primitive(dt)) if !matches!(dt, DataType::Null)
        )
    });

    if has_class && has_null {
        return Err(PhpTypeParseError::ClassNullableNotRepresentable);
    }
    if has_class && has_non_null_primitive {
        return Err(PhpTypeParseError::MixedPrimitiveAndClass);
    }

    if has_group {
        let mut terms: Vec<DnfTerm> = Vec::with_capacity(alts.len());
        for (alt, _) in alts {
            terms.push(match alt {
                Alt::Group(names) => DnfTerm::Intersection(names),
                Alt::Atom(Atom::Class(name)) => DnfTerm::Single(name),
                Alt::Atom(Atom::Primitive(_)) => {
                    unreachable!("guarded above by has_class && has_*_primitive checks")
                }
            });
        }
        check_no_duplicate_in_dnf(&terms)?;
        return Ok(PhpType::Dnf(terms));
    }

    if !has_class {
        let members: Vec<DataType> = alts
            .into_iter()
            .map(|(alt, _)| match alt {
                Alt::Atom(Atom::Primitive(dt)) => dt,
                _ => unreachable!("class-free path"),
            })
            .collect();
        check_no_duplicate_data_types(&members)?;
        return Ok(PhpType::Union(members));
    }

    let names: Vec<String> = alts
        .into_iter()
        .map(|(alt, _)| match alt {
            Alt::Atom(Atom::Class(name)) => name,
            _ => unreachable!("primitive-free path"),
        })
        .collect();
    check_no_duplicate_strings(&names)?;
    Ok(PhpType::ClassUnion(names))
}

fn check_no_duplicate_data_types(members: &[DataType]) -> Result<(), PhpTypeParseError> {
    for (i, a) in members.iter().enumerate() {
        for b in &members[..i] {
            if a == b {
                return Err(PhpTypeParseError::DuplicateMember {
                    name: format!("{a}"),
                });
            }
        }
    }
    Ok(())
}

fn check_no_duplicate_strings(names: &[String]) -> Result<(), PhpTypeParseError> {
    for (i, a) in names.iter().enumerate() {
        for b in &names[..i] {
            if a == b {
                return Err(PhpTypeParseError::DuplicateMember { name: a.clone() });
            }
        }
    }
    Ok(())
}

fn check_no_duplicate_in_dnf(terms: &[DnfTerm]) -> Result<(), PhpTypeParseError> {
    for (i, a) in terms.iter().enumerate() {
        for b in &terms[..i] {
            if a == b {
                let name = match a {
                    DnfTerm::Single(s) => s.clone(),
                    DnfTerm::Intersection(parts) => format!("({})", parts.join("&")),
                };
                return Err(PhpTypeParseError::DuplicateMember { name });
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Alt {
    Atom(Atom),
    Group(Vec<String>),
}

fn parse_alt(raw: &str, span_start: usize) -> Result<Alt, PhpTypeParseError> {
    let trimmed = raw.trim();
    if trimmed.starts_with('(') {
        let leading = raw.len() - raw.trim_start().len();
        let group_start = span_start + leading;
        return parse_group(trimmed, group_start).map(Alt::Group);
    }
    if trimmed.contains('&') {
        let amp_pos = raw.find('&').map_or(span_start, |i| span_start + i);
        return Err(PhpTypeParseError::NakedAmpInUnion { pos: amp_pos });
    }
    parse_atom(trimmed).map(Alt::Atom)
}

fn parse_group(raw: &str, group_start: usize) -> Result<Vec<String>, PhpTypeParseError> {
    debug_assert!(raw.starts_with('('));
    let inner_end = match raw.rfind(')') {
        Some(i) if i > 0 => i,
        _ => {
            return Err(PhpTypeParseError::UnbalancedParens { pos: group_start });
        }
    };
    let after_close = raw[inner_end + 1..].trim();
    if !after_close.is_empty() {
        return Err(PhpTypeParseError::UnexpectedChar {
            ch: after_close.chars().next().unwrap_or(')'),
            pos: group_start + inner_end + 1,
        });
    }
    let inner = &raw[1..inner_end];
    let inner_offset = group_start + 1;
    if inner.contains('(') {
        return Err(PhpTypeParseError::NestedGroups {
            pos: inner_offset + inner.find('(').unwrap_or(0),
        });
    }
    if has_top_level_char(inner, '|') {
        let pipe_pos = inner.find('|').map_or(inner_offset, |i| inner_offset + i);
        return Err(PhpTypeParseError::UnionInIntersection { pos: pipe_pos });
    }

    let pieces = split_top_level_amps(inner);
    if pieces.len() < 2 {
        return Err(PhpTypeParseError::IntersectionTooSmall { pos: group_start });
    }
    let mut names: Vec<String> = Vec::with_capacity(pieces.len());
    for piece in pieces {
        let span_start = inner_offset + piece.start;
        let part = &inner[piece.start..piece.end];
        if part.trim().is_empty() {
            return Err(PhpTypeParseError::EmptyTerm { pos: span_start });
        }
        match parse_atom(part)? {
            Atom::Class(name) => names.push(name),
            Atom::Primitive(dt) => {
                return Err(PhpTypeParseError::PrimitiveInIntersection {
                    name: format!("{dt}"),
                });
            }
        }
    }
    Ok(names)
}

fn parse_bare_intersection(body: &str, body_offset: usize) -> Result<PhpType, PhpTypeParseError> {
    let mut names: Vec<String> = Vec::new();
    for piece in split_top_level_amps(body) {
        let span_start = body_offset + piece.start;
        let raw = &body[piece.start..piece.end];
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(PhpTypeParseError::EmptyTerm { pos: span_start });
        }
        if trimmed.starts_with('(') {
            // `A&(...)` — intersections cannot contain a paren group at all.
            // The inner shape is a union (`A&(B|C)`) or another intersection
            // (`A&(B&C)`); both are illegal in PHP type hints.
            let leading_ws = raw.len() - raw.trim_start().len();
            return Err(PhpTypeParseError::UnionInIntersection {
                pos: span_start + leading_ws,
            });
        }
        match parse_atom(raw)? {
            Atom::Class(name) => names.push(name),
            Atom::Primitive(dt) => {
                return Err(PhpTypeParseError::PrimitiveInIntersection {
                    name: format!("{dt}"),
                });
            }
        }
    }
    check_no_duplicate_strings(&names)?;
    Ok(PhpType::Intersection(names))
}

fn split_top_level_amps(body: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, ch) in body.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            '&' if depth == 0 => {
                pieces.push(Piece { start, end: i });
                start = i + 1;
            }
            _ => {}
        }
    }
    pieces.push(Piece {
        start,
        end: body.len(),
    });
    pieces
}

#[derive(Debug, Clone, Copy)]
struct Piece {
    start: usize,
    end: usize,
}

fn split_top_level_pipes(body: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, ch) in body.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            '|' if depth == 0 => {
                pieces.push(Piece { start, end: i });
                start = i + 1;
            }
            _ => {}
        }
    }
    pieces.push(Piece {
        start,
        end: body.len(),
    });
    pieces
}

fn reject_unsupported_keyword(name: &str) -> Result<(), PhpTypeParseError> {
    let lowered = name.to_ascii_lowercase();
    match lowered.as_str() {
        "static" | "never" | "self" | "parent" => Err(PhpTypeParseError::UnsupportedKeyword {
            name: name.to_owned(),
        }),
        _ => Ok(()),
    }
}

fn normalise_class_name(raw: &str) -> Result<String, PhpTypeParseError> {
    let stripped = raw.strip_prefix('\\').unwrap_or(raw);
    if stripped.is_empty() || stripped.contains('\0') {
        return Err(PhpTypeParseError::InvalidClassName {
            name: raw.to_owned(),
        });
    }
    Ok(stripped.to_owned())
}

fn primitive_from_name(name: &str) -> Option<DataType> {
    let lowered = name.to_ascii_lowercase();
    Some(match lowered.as_str() {
        "int" => DataType::Long,
        "float" => DataType::Double,
        "bool" => DataType::Bool,
        "true" => DataType::True,
        "false" => DataType::False,
        "string" => DataType::String,
        "array" => DataType::Array,
        "object" => DataType::Object(None),
        "callable" => DataType::Callable,
        "iterable" => DataType::Iterable,
        "resource" => DataType::Resource,
        "mixed" => DataType::Mixed,
        "void" => DataType::Void,
        "null" => DataType::Null,
        _ => return None,
    })
}

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

    #[test]
    fn parses_int_primitive() {
        let ty: PhpType = "int".parse().expect("int parses");
        assert_eq!(ty, PhpType::Simple(DataType::Long));
    }

    #[test]
    fn parses_every_primitive_name() {
        let cases: &[(&str, DataType)] = &[
            ("int", DataType::Long),
            ("float", DataType::Double),
            ("bool", DataType::Bool),
            ("true", DataType::True),
            ("false", DataType::False),
            ("string", DataType::String),
            ("array", DataType::Array),
            ("object", DataType::Object(None)),
            ("callable", DataType::Callable),
            ("iterable", DataType::Iterable),
            ("resource", DataType::Resource),
            ("mixed", DataType::Mixed),
            ("void", DataType::Void),
            ("null", DataType::Null),
        ];
        for &(name, expected) in cases {
            let parsed: PhpType = name.parse().unwrap_or_else(|e| panic!("{name} → {e}"));
            assert_eq!(parsed, PhpType::Simple(expected), "name = {name}");
        }
    }

    #[test]
    fn primitives_are_case_insensitive() {
        for input in ["INT", "Int", "iNt"] {
            let parsed: PhpType = input.parse().expect("case insensitive");
            assert_eq!(parsed, PhpType::Simple(DataType::Long), "input = {input}");
        }
    }

    #[test]
    fn parses_single_class_into_class_union() {
        let parsed: PhpType = "Foo".parse().expect("class parses");
        assert_eq!(parsed, PhpType::ClassUnion(vec!["Foo".to_owned()]));
    }

    #[test]
    fn strips_leading_backslash_from_class_name() {
        let parsed: PhpType = "\\Foo".parse().expect("\\Foo parses");
        assert_eq!(parsed, PhpType::ClassUnion(vec!["Foo".to_owned()]));
    }

    #[test]
    fn preserves_namespace_separators() {
        let parsed: PhpType = "\\Ns\\Foo".parse().expect("namespaced class parses");
        assert_eq!(parsed, PhpType::ClassUnion(vec!["Ns\\Foo".to_owned()]));
    }

    #[test]
    fn class_names_keep_their_case() {
        let parsed: PhpType = "FooBar".parse().expect("CamelCase preserved");
        assert_eq!(parsed, PhpType::ClassUnion(vec!["FooBar".to_owned()]));
    }

    #[test]
    fn parses_primitive_union() {
        let parsed: PhpType = "int|string".parse().expect("union parses");
        assert_eq!(
            parsed,
            PhpType::Union(vec![DataType::Long, DataType::String])
        );
    }

    #[test]
    fn parses_primitive_union_with_inline_null() {
        let parsed: PhpType = "int|string|null".parse().expect("nullable union parses");
        assert_eq!(
            parsed,
            PhpType::Union(vec![DataType::Long, DataType::String, DataType::Null])
        );
    }

    #[test]
    fn nullable_shorthand_canonicalises_to_union_for_primitives() {
        let parsed: PhpType = "?int".parse().expect("?int parses");
        assert_eq!(parsed, PhpType::Union(vec![DataType::Long, DataType::Null]));
    }

    #[test]
    fn whitespace_around_pipes_is_tolerated() {
        let parsed: PhpType = "int | string".parse().expect("whitespace tolerated");
        assert_eq!(
            parsed,
            PhpType::Union(vec![DataType::Long, DataType::String])
        );
    }

    #[test]
    fn parses_class_union() {
        let parsed: PhpType = "Foo|Bar".parse().expect("class union parses");
        assert_eq!(
            parsed,
            PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()])
        );
    }

    #[test]
    fn class_union_strips_backslashes_per_member() {
        let parsed: PhpType = "\\Foo|\\Ns\\Bar".parse().expect("class union normalises");
        assert_eq!(
            parsed,
            PhpType::ClassUnion(vec!["Foo".to_owned(), "Ns\\Bar".to_owned()])
        );
    }

    #[test]
    fn parses_bare_intersection() {
        let parsed: PhpType = "Foo&Bar".parse().expect("intersection parses");
        assert_eq!(
            parsed,
            PhpType::Intersection(vec!["Foo".to_owned(), "Bar".to_owned()])
        );
    }

    #[test]
    fn parses_three_way_bare_intersection() {
        let parsed: PhpType = "A&B&C".parse().expect("3-way intersection parses");
        assert_eq!(
            parsed,
            PhpType::Intersection(vec!["A".to_owned(), "B".to_owned(), "C".to_owned()])
        );
    }

    #[test]
    fn parses_dnf_group_then_single() {
        let parsed: PhpType = "(A&B)|C".parse().expect("(A&B)|C parses");
        assert_eq!(
            parsed,
            PhpType::Dnf(vec![
                DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
                DnfTerm::Single("C".to_owned()),
            ])
        );
    }

    #[test]
    fn parses_dnf_single_then_group() {
        let parsed: PhpType = "C|(A&B)".parse().expect("C|(A&B) parses");
        assert_eq!(
            parsed,
            PhpType::Dnf(vec![
                DnfTerm::Single("C".to_owned()),
                DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
            ])
        );
    }

    #[test]
    fn parses_dnf_group_then_two_singles() {
        let parsed: PhpType = "(A&B)|C|D".parse().expect("(A&B)|C|D parses");
        assert_eq!(
            parsed,
            PhpType::Dnf(vec![
                DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
                DnfTerm::Single("C".to_owned()),
                DnfTerm::Single("D".to_owned()),
            ])
        );
    }

    #[test]
    fn parses_dnf_group_strips_backslashes() {
        let parsed: PhpType = "(\\A&\\B)|\\C".parse().expect("(\\A&\\B)|\\C parses");
        assert_eq!(
            parsed,
            PhpType::Dnf(vec![
                DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
                DnfTerm::Single("C".to_owned()),
            ])
        );
    }

    fn err(input: &str) -> PhpTypeParseError {
        input.parse::<PhpType>().expect_err(input)
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(err(""), PhpTypeParseError::Empty);
        assert_eq!(err("   "), PhpTypeParseError::Empty);
    }

    #[test]
    fn rejects_leading_pipe() {
        assert!(matches!(err("|int"), PhpTypeParseError::EmptyTerm { .. }));
    }

    #[test]
    fn rejects_trailing_pipe() {
        assert!(matches!(err("int|"), PhpTypeParseError::EmptyTerm { .. }));
    }

    #[test]
    fn rejects_double_pipe() {
        assert!(matches!(
            err("int||string"),
            PhpTypeParseError::EmptyTerm { .. }
        ));
    }

    #[test]
    fn rejects_unbalanced_paren() {
        assert!(matches!(
            err("(A&B|C"),
            PhpTypeParseError::UnbalancedParens { .. }
        ));
    }

    #[test]
    fn rejects_union_inside_intersection() {
        assert!(matches!(
            err("A&(B|C)"),
            PhpTypeParseError::NakedAmpInUnion { .. }
                | PhpTypeParseError::UnionInIntersection { .. }
        ));
    }

    #[test]
    fn rejects_naked_amp_in_union() {
        assert!(matches!(
            err("A&B|C"),
            PhpTypeParseError::NakedAmpInUnion { .. }
        ));
    }

    #[test]
    fn rejects_nullable_compound_union() {
        assert!(matches!(
            err("?int|string"),
            PhpTypeParseError::NullableCompound { .. }
        ));
    }

    #[test]
    fn rejects_nullable_compound_intersection() {
        assert!(matches!(
            err("?A&B"),
            PhpTypeParseError::NullableCompound { .. }
        ));
    }

    #[test]
    fn rejects_unsupported_keywords() {
        for kw in ["static", "never", "self", "parent"] {
            assert!(
                matches!(err(kw), PhpTypeParseError::UnsupportedKeyword { .. }),
                "{kw} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_class_nullable_simple() {
        assert_eq!(
            err("?Foo"),
            PhpTypeParseError::ClassNullableNotRepresentable
        );
    }

    #[test]
    fn rejects_class_nullable_pipe_null() {
        assert_eq!(
            err("Foo|null"),
            PhpTypeParseError::ClassNullableNotRepresentable
        );
    }

    #[test]
    fn rejects_class_union_with_null_member() {
        assert_eq!(
            err("Foo|Bar|null"),
            PhpTypeParseError::ClassNullableNotRepresentable
        );
    }

    #[test]
    fn rejects_dnf_with_null_member() {
        assert_eq!(
            err("(A&B)|null"),
            PhpTypeParseError::ClassNullableNotRepresentable
        );
    }

    #[test]
    fn rejects_mixed_primitive_and_class() {
        assert_eq!(err("int|Foo"), PhpTypeParseError::MixedPrimitiveAndClass);
    }

    #[test]
    fn rejects_single_element_paren_group() {
        assert!(matches!(
            err("(A)|B"),
            PhpTypeParseError::IntersectionTooSmall { .. }
        ));
    }

    #[test]
    fn rejects_primitive_in_intersection() {
        assert!(matches!(
            err("A&int"),
            PhpTypeParseError::PrimitiveInIntersection { .. }
        ));
        assert!(matches!(
            err("(A&int)|C"),
            PhpTypeParseError::PrimitiveInIntersection { .. }
        ));
    }

    #[test]
    fn rejects_duplicate_in_union() {
        assert!(matches!(
            err("int|int"),
            PhpTypeParseError::DuplicateMember { .. }
        ));
    }

    #[test]
    fn rejects_duplicate_in_class_union() {
        assert!(matches!(
            err("Foo|Foo"),
            PhpTypeParseError::DuplicateMember { .. }
        ));
    }

    #[test]
    fn rejects_duplicate_in_intersection() {
        assert!(matches!(
            err("A&B&A"),
            PhpTypeParseError::DuplicateMember { .. }
        ));
    }

    #[test]
    fn rejects_duplicate_in_dnf() {
        assert!(matches!(
            err("(A&B)|C|C"),
            PhpTypeParseError::DuplicateMember { .. }
        ));
    }

    #[test]
    fn display_simple_primitives_match_php_names() {
        let cases: &[(DataType, &str)] = &[
            (DataType::Long, "int"),
            (DataType::Double, "float"),
            (DataType::Bool, "bool"),
            (DataType::True, "true"),
            (DataType::False, "false"),
            (DataType::String, "string"),
            (DataType::Array, "array"),
            (DataType::Object(None), "object"),
            (DataType::Callable, "callable"),
            (DataType::Iterable, "iterable"),
            (DataType::Resource, "resource"),
            (DataType::Mixed, "mixed"),
            (DataType::Void, "void"),
            (DataType::Null, "null"),
        ];
        for &(dt, expected) in cases {
            let s = format!("{}", PhpType::Simple(dt));
            assert_eq!(s, expected, "DataType::{dt:?}");
        }
    }

    #[test]
    fn display_class_union_adds_leading_backslash() {
        let ty = PhpType::ClassUnion(vec!["Foo".to_owned(), "Ns\\Bar".to_owned()]);
        assert_eq!(format!("{ty}"), "\\Foo|\\Ns\\Bar");
    }

    #[test]
    fn display_intersection_renders_amp_separated() {
        let ty = PhpType::Intersection(vec!["A".to_owned(), "B".to_owned()]);
        assert_eq!(format!("{ty}"), "\\A&\\B");
    }

    #[test]
    fn display_dnf_wraps_intersection_groups_in_parens() {
        let ty = PhpType::Dnf(vec![
            DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
            DnfTerm::Single("C".to_owned()),
        ]);
        assert_eq!(format!("{ty}"), "(\\A&\\B)|\\C");
    }

    #[test]
    fn display_union_pipe_separated_with_inline_null() {
        let ty = PhpType::Union(vec![DataType::Long, DataType::String, DataType::Null]);
        assert_eq!(format!("{ty}"), "int|string|null");
    }

    #[test]
    fn display_already_qualified_class_does_not_double_backslash() {
        let ty = PhpType::ClassUnion(vec!["\\AlreadyQualified".to_owned()]);
        assert_eq!(format!("{ty}"), "\\AlreadyQualified");
    }

    #[test]
    fn roundtrip_happy_path_corpus() {
        let inputs = [
            "int",
            "string",
            "bool",
            "void",
            "null",
            "object",
            "iterable",
            "callable",
            "Foo",
            "\\Foo",
            "\\Ns\\Foo",
            "int|string",
            "int|string|null",
            "?int",
            "Foo|Bar",
            "\\Foo|\\Bar",
            "Foo&Bar",
            "A&B&C",
            "(A&B)|C",
            "C|(A&B)",
            "(A&B)|C|D",
            "(\\A&\\B)|\\C",
            "int | string",
        ];
        for input in inputs {
            let parsed: PhpType = input.parse().unwrap_or_else(|e| panic!("{input} → {e}"));
            let rendered = format!("{parsed}");
            let reparsed: PhpType = rendered
                .parse()
                .unwrap_or_else(|e| panic!("reparse {rendered} → {e}"));
            assert_eq!(
                parsed, reparsed,
                "input {input:?} rendered as {rendered:?} did not roundtrip"
            );
        }
    }
}
