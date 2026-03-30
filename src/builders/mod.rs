//! Structures that are used to construct other, more complicated types.
//! Generally zero-cost abstractions.

mod class;
#[cfg(feature = "enum")]
mod enum_builder;
mod function;
#[cfg(all(php82, feature = "embed"))]
mod ini;
mod module;
#[cfg(feature = "embed")]
mod sapi;

pub use class::{ClassBuilder, ClassProperty};
#[cfg(feature = "enum")]
pub use enum_builder::EnumBuilder;
pub use function::FunctionBuilder;
#[cfg(all(php82, feature = "embed"))]
pub use ini::IniBuilder;
pub use module::{ModuleBuilder, ModuleStartup};
#[cfg(feature = "embed")]
pub use sapi::SapiBuilder;
