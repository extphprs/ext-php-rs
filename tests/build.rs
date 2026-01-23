//! Build script for the tests crate that detects PHP version and sets cfg flags.
//!
//! This uses the shared `ext-php-rs-build` crate to detect the PHP version
//! and emit the appropriate cfg flags for conditional compilation.

use ext_php_rs_build::{ApiVersion, PHPInfo, emit_check_cfg, emit_rerun_if_env_changed, find_php};

fn main() {
    // Declare check-cfg for all PHP version flags
    emit_check_cfg();

    // Rerun if PHP environment changes
    emit_rerun_if_env_changed();

    let Ok(php) = find_php() else {
        eprintln!("Warning: Could not find PHP executable, version-specific tests may not run");
        return;
    };

    let Ok(info) = PHPInfo::get(&php) else {
        eprintln!("Warning: Could not get PHP info, version-specific tests may not run");
        return;
    };

    let Ok(zend_version) = info.zend_version() else {
        eprintln!("Warning: Could not get PHP API version, version-specific tests may not run");
        return;
    };

    let Ok(version) = ApiVersion::try_from(zend_version) else {
        eprintln!("Warning: Unsupported PHP version, version-specific tests may not run");
        return;
    };

    // Emit cfg flags for all supported API versions
    for supported_version in version.supported_apis() {
        println!("cargo:rustc-cfg={}", supported_version.cfg_name());
    }
}
