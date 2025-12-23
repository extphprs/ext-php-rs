//! Build script for the tests crate that detects PHP version and sets cfg flags.
//!
//! This mirrors the PHP version detection in ext-php-rs's build.rs to ensure
//! conditional compilation flags like `php82` are set correctly for the test code.

use std::path::PathBuf;
use std::process::Command;

/// Finds the location of an executable `name`.
fn find_executable(name: &str) -> Option<PathBuf> {
    const WHICH: &str = if cfg!(windows) { "where" } else { "which" };
    let cmd = Command::new(WHICH).arg(name).output().ok()?;
    if cmd.status.success() {
        let stdout = String::from_utf8_lossy(&cmd.stdout);
        stdout.trim().lines().next().map(|l| l.trim().into())
    } else {
        None
    }
}

/// Finds the location of the PHP executable.
fn find_php() -> Option<PathBuf> {
    // If path is given via env, it takes priority.
    if let Some(path) = std::env::var_os("PHP").map(PathBuf::from)
        && path.try_exists().unwrap_or(false)
    {
        return Some(path);
    }
    find_executable("php")
}

/// Get PHP version as a (major, minor) tuple.
fn get_php_version() -> Option<(u32, u32)> {
    let php = find_php()?;
    let output = Command::new(&php)
        .arg("-r")
        .arg("echo PHP_MAJOR_VERSION . '.' . PHP_MINOR_VERSION;")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = version_str.trim().split('.').collect();
    if parts.len() >= 2 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        Some((major, minor))
    } else {
        None
    }
}

fn main() {
    // Declare check-cfg for all PHP version flags
    println!("cargo::rustc-check-cfg=cfg(php80, php81, php82, php83, php84, php85)");

    // Rerun if PHP environment changes
    println!("cargo:rerun-if-env-changed=PHP");
    println!("cargo:rerun-if-env-changed=PATH");

    let Some((major, minor)) = get_php_version() else {
        eprintln!("Warning: Could not detect PHP version, DNF tests may not run");
        return;
    };

    // Set cumulative version flags (like ext-php-rs does)
    // PHP 8.0 is baseline, no flag needed
    if major >= 8 {
        if minor >= 1 {
            println!("cargo:rustc-cfg=php81");
        }
        if minor >= 2 {
            println!("cargo:rustc-cfg=php82");
        }
        if minor >= 3 {
            println!("cargo:rustc-cfg=php83");
        }
        if minor >= 4 {
            println!("cargo:rustc-cfg=php84");
        }
        if minor >= 5 {
            println!("cargo:rustc-cfg=php85");
        }
    }
}
