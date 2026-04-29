//! `PhpUnion` trait for Rust enums that map to PHP unions.
//!
//! [`PhpUnion`] is the runtime hook used by the
//! [`#[derive(PhpUnion)]`](ext_php_rs_derive::PhpUnion) macro to expose the
//! [`PhpType`] of a Rust enum whose variants newtype-wrap distinct PHP types.
//! Authors do not implement [`PhpUnion`] manually; the derive produces the
//! impl alongside [`IntoZval`](crate::convert::IntoZval) and
//! [`FromZval`](crate::convert::FromZval) so the enum can be used directly as
//! an `#[php_function]` parameter and return type.
//!
//! # Example
//!
//! ```rust,ignore
//! use ext_php_rs::types::PhpUnion;
//! use ext_php_rs::ZvalConvert;
//!
//! # use ext_php_rs::types::{PhpType};
//! # use ext_php_rs::flags::DataType;
//! #[derive(ext_php_rs::PhpUnion)]
//! pub enum IntOrString {
//!     Int(i64),
//!     String(String),
//! }
//!
//! assert_eq!(
//!     <IntOrString as PhpUnion>::union_types(),
//!     PhpType::Union(vec![DataType::Long, DataType::String]),
//! );
//! ```

use crate::types::PhpType;

/// A Rust enum whose variants newtype-wrap the members of a PHP union.
///
/// Implemented by the [`#[derive(PhpUnion)]`](ext_php_rs_derive::PhpUnion)
/// macro. The function macro consults [`PhpUnion::union_types`] (via the
/// `php_type()` override on [`IntoZval`](crate::convert::IntoZval) /
/// [`FromZval`](crate::convert::FromZval)) to register the correct
/// [`PhpType::Union`] on the underlying [`Arg`](crate::args::Arg).
pub trait PhpUnion {
    /// The [`PhpType`] this enum represents.
    ///
    /// For an enum whose variants wrap `i64` and `String`, this returns
    /// `PhpType::Union(vec![DataType::Long, DataType::String])`.
    fn union_types() -> PhpType;
}
