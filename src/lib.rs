#![doc = include_str!("../README.md")]
#![deny(clippy::unwrap_used)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unsafe_attr_outside_unsafe)]
#![warn(clippy::pedantic)]
#![cfg_attr(docs, feature(doc_cfg))]
#![cfg_attr(windows, feature(abi_vectorcall))]

pub mod alloc;
pub mod args;
pub mod binary;
pub mod binary_slice;
pub mod builders;
pub mod convert;
pub mod error;
pub mod exception;
pub mod ffi;
pub mod flags;
#[macro_use]
pub mod macros;
pub mod boxed;
pub mod class;
#[cfg(any(docs, feature = "closure"))]
#[cfg_attr(docs, doc(cfg(feature = "closure")))]
pub mod closure;
pub mod constant;
pub mod describe;
#[cfg(feature = "embed")]
pub mod embed;
#[cfg(feature = "enum")]
pub mod enum_;
#[cfg(feature = "observer")]
#[cfg_attr(docs, doc(cfg(feature = "observer")))]
pub mod observer {
    //! Observer API for function call profiling and tracing.
    pub use crate::zend::observer::{FcallInfo, FcallObserver};
}
#[doc(hidden)]
pub mod internal;

// Re-export inventory for use by macros
#[doc(hidden)]
pub use inventory;
pub mod props;
pub mod rc;
#[cfg(test)]
pub mod test;
pub mod types;
mod util;
pub mod zend;

/// A module typically glob-imported containing the typically required macros
/// and imports.
pub mod prelude {

    pub use crate::builders::ModuleBuilder;
    #[cfg(any(docs, feature = "closure"))]
    #[cfg_attr(docs, doc(cfg(feature = "closure")))]
    pub use crate::closure::Closure;
    pub use crate::exception::{PhpException, PhpResult};
    #[cfg(feature = "enum")]
    pub use crate::php_enum;
    pub use crate::php_print;
    pub use crate::php_println;
    pub use crate::php_write;
    pub use crate::types::ZendCallable;
    pub use crate::zend::BailoutGuard;
    #[cfg(feature = "observer")]
    pub use crate::zend::{FcallInfo, FcallObserver};
    pub use crate::{
        ZvalConvert, php_class, php_const, php_extern, php_function, php_impl, php_impl_interface,
        php_interface, php_module, wrap_constant, wrap_function, zend_fastcall,
    };
}

/// `ext-php-rs` version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Whether the extension is compiled for PHP debug mode.
pub const PHP_DEBUG: bool = cfg!(php_debug);

/// Whether the extension is compiled for PHP thread-safe mode.
pub const PHP_ZTS: bool = cfg!(php_zts);

/// Whether the extension is compiled for PHP 8.1 or later.
pub const PHP_81: bool = cfg!(php81);

/// Whether the extension is compiled for PHP 8.2 or later.
pub const PHP_82: bool = cfg!(php82);

/// Whether the extension is compiled for PHP 8.3 or later.
pub const PHP_83: bool = cfg!(php83);

/// Whether the extension is compiled for PHP 8.4 or later.
pub const PHP_84: bool = cfg!(php84);

/// Whether the extension is compiled for PHP 8.5 or later.
pub const PHP_85: bool = cfg!(php85);

#[cfg(feature = "enum")]
pub use ext_php_rs_derive::php_enum;
pub use ext_php_rs_derive::{
    ZvalConvert, php_class, php_const, php_extern, php_function, php_impl, php_impl_interface,
    php_interface, php_module, wrap_constant, wrap_function, zend_fastcall,
};
