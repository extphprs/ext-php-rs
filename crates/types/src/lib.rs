//! Shared PHP type-string parser and AST for [`ext-php-rs`].
//!
//! This crate hosts the type-system primitives that need to be reachable
//! both from the runtime crate ([`ext-php-rs`]) and from the proc-macro
//! crate ([`ext-php-rs-derive`]). Cargo cannot resolve a dependency cycle
//! between those two, so the shared pieces — [`PhpType`], [`DnfTerm`],
//! [`DataType`], [`PhpTypeParseError`], and the [`FromStr`][std::str::FromStr]
//! impl on [`PhpType`] — live in this third crate.
//!
//! When the `proc-macro` feature is enabled, [`PhpType`], [`DnfTerm`], and
//! [`DataType`] also implement [`quote::ToTokens`], so the macro crate can
//! parse a `LitStr` at expansion time and emit a literal value of the parsed
//! shape. The runtime crate keeps the feature off; consumers pay nothing.
//!
//! [`ext-php-rs`]: https://crates.io/crates/ext-php-rs
//! [`ext-php-rs-derive`]: https://crates.io/crates/ext-php-rs-derive

#![cfg_attr(docsrs, feature(doc_cfg))]

mod data_type;
mod php_type;

pub use data_type::DataType;
pub use php_type::{DnfTerm, PhpType, PhpTypeParseError};
