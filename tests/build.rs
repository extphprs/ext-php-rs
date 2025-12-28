//! Build script for the tests crate that registers PHP version cfg flags.
//!
//! This reads the `DEP_EXT_PHP_RS_*` environment variables set by ext-php-rs's
//! build script and emits the corresponding `cargo:rustc-cfg` directives.
//!
//! Note: This duplicates logic from `ext_php_rs::build::register_php_cfg_flags()`
//! because using ext-php-rs as a build-dependency causes duplicate artifact issues
//! in workspace builds. Keep this in sync with `src/build.rs`.

/// PHP version cfg flags that can be set.
/// Keep in sync with `PHP_CFG_FLAGS` in `src/build.rs`.
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

fn main() {
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
