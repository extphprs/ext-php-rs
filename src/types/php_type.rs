//! Re-export shim for the type-string vocabulary.
//!
//! [`PhpType`], [`DnfTerm`], and [`PhpTypeParseError`] are defined in the
//! [`ext-php-rs-types`](https://crates.io/crates/ext-php-rs-types) workspace
//! member so the proc-macro crate can call the parser at expansion time
//! without re-introducing a dependency cycle on this runtime crate.
//!
//! User code keeps using `ext_php_rs::types::{PhpType, DnfTerm}`; this file
//! is the public address those names live at.

pub use ext_php_rs_types::{DnfTerm, PhpType, PhpTypeParseError};
