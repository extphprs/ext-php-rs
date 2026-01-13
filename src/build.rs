//! Build script helpers for extension crates.
//!
//! This module provides utilities for extension crate build scripts to
//! propagate PHP version cfg flags from ext-php-rs. This is **optional** and
//! only needed if you want to use conditional compilation based on PHP version
//! (e.g., `#[cfg(php82)]`).
//!
//! # When You Need This
//!
//! You only need to use this module if:
//! - You want to conditionally compile code based on PHP version
//! - You use features that require specific PHP versions (like `readonly` classes
//!   which require PHP 8.2+) and want to support multiple PHP versions
//!
//! If your extension only targets a single PHP version, you don't need this.
//!
//! # Example
//!
//! In your extension's `Cargo.toml`:
//!
//! ```toml
//! [build-dependencies]
//! ext-php-rs = "0.15"
//! ```
//!
//! In your extension's `build.rs`:
//!
//! ```ignore
//! fn main() {
//!     ext_php_rs::build::register_php_cfg_flags();
//! }
//! ```
//!
//! Then in your code you can use conditional compilation:
//!
//! ```ignore
//! #[php_class]
//! #[cfg_attr(php82, php(readonly))]
//! pub struct MyClass {
//!     // ...
//! }
//! ```
//!
//! # Available Flags
//!
//! The following cfg flags are available after calling [`register_php_cfg_flags`]:
//!
//! - `php80`, `php81`, `php82`, `php83`, `php84`, `php85` - PHP version flags
//!   (cumulative: if `php83` is set, `php82` and `php81` are also set)
//! - `php_debug` - Set when PHP was built with debug mode
//! - `php_zts` - Set when PHP was built with thread safety (ZTS)

/// PHP version cfg flags that can be set.
const PHP_CFG_FLAGS: &[&str] = &[
    "PHP80",
    "PHP81",
    "PHP82",
    "PHP83",
    "PHP84",
    "PHP85",
    "PHP_DEBUG",
    "PHP_ZTS",
];

/// Registers PHP version cfg flags for the current crate.
///
/// This function reads the `DEP_EXT_PHP_RS_*` environment variables set by
/// ext-php-rs's build script and emits the corresponding `cargo:rustc-cfg`
/// directives for the current crate.
///
/// Call this from your extension's `build.rs` to enable conditional compilation
/// based on PHP version (e.g., `#[cfg(php82)]`).
///
/// # Example
///
/// ```ignore
/// // build.rs
/// fn main() {
///     ext_php_rs::build::register_php_cfg_flags();
/// }
/// ```
///
/// Then in your code:
///
/// ```ignore
/// #[cfg(php82)]
/// #[php_class(readonly)]
/// pub struct MyReadonlyClass {
///     // ...
/// }
/// ```
pub fn register_php_cfg_flags() {
    // Register valid cfg values with cargo
    println!(
        "cargo::rustc-check-cfg=cfg(php80, php81, php82, php83, php84, php85, php_zts, php_debug)"
    );

    // Read DEP_EXT_PHP_RS_* env vars and emit cfg flags
    for flag in PHP_CFG_FLAGS {
        let env_var = format!("DEP_EXT_PHP_RS_{flag}");
        if std::env::var(&env_var).is_ok() {
            println!("cargo:rustc-cfg={}", flag.to_lowercase());
        }
    }
}
