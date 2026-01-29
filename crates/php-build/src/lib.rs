//! Build-time PHP detection utilities for ext-php-rs.
//!
//! This crate provides utilities for detecting PHP installations and version
//! information at build time. It is used by ext-php-rs's build script and can
//! be used by other crates that need to detect PHP at compile time.
//!
//! # Example
//!
//! ```no_run
//! use ext_php_rs_build::{find_php, PHPInfo, ApiVersion};
//!
//! fn main() -> anyhow::Result<()> {
//!     let php = find_php()?;
//!     let info = PHPInfo::get(&php)?;
//!     let version: ApiVersion = info.zend_version()?.try_into()?;
//!
//!     for api in version.supported_apis() {
//!         println!("cargo:rustc-cfg={}", api.cfg_name());
//!     }
//!     Ok(())
//! }
//! ```

use std::{
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

#[cfg(windows)]
use std::fmt::Display;

use anyhow::{Context, Error, Result, bail};

/// Finds the location of an executable `name`.
#[must_use]
pub fn find_executable(name: &str) -> Option<PathBuf> {
    const WHICH: &str = if cfg!(windows) { "where" } else { "which" };
    let cmd = Command::new(WHICH).arg(name).output().ok()?;
    if cmd.status.success() {
        let stdout = String::from_utf8_lossy(&cmd.stdout);
        stdout.trim().lines().next().map(|l| l.trim().into())
    } else {
        None
    }
}

/// Returns an environment variable's value as a `PathBuf`.
#[must_use]
pub fn path_from_env(key: &str) -> Option<PathBuf> {
    std::env::var_os(key).map(PathBuf::from)
}

/// Finds the location of the PHP executable.
///
/// # Errors
///
/// Returns an error if PHP cannot be found.
pub fn find_php() -> Result<PathBuf> {
    // If path is given via env, it takes priority.
    if let Some(path) = path_from_env("PHP") {
        if !path.try_exists()? {
            // If path was explicitly given and it can't be found, this is a hard error
            bail!("php executable not found at {}", path.display());
        }
        return Ok(path);
    }
    find_executable("php").with_context(|| {
        "Could not find PHP executable. \
        Please ensure `php` is in your PATH or the `PHP` environment variable is set."
    })
}

/// Output of `php -i`.
pub struct PHPInfo(String);

impl PHPInfo {
    /// Get the PHP info by running `php -i`.
    ///
    /// # Errors
    ///
    /// Returns an error if `php -i` command failed to execute successfully.
    pub fn get(php: &Path) -> Result<Self> {
        let cmd = Command::new(php)
            .arg("-i")
            .output()
            .context("Failed to call `php -i`")?;
        if !cmd.status.success() {
            bail!("Failed to call `php -i` status code {}", cmd.status);
        }
        let stdout = String::from_utf8_lossy(&cmd.stdout);
        Ok(Self(stdout.to_string()))
    }

    /// Checks if thread safety is enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if `PHPInfo` does not contain thread safety information.
    pub fn thread_safety(&self) -> Result<bool> {
        Ok(self
            .get_key("Thread Safety")
            .context("Could not find thread safety of PHP")?
            == "enabled")
    }

    /// Checks if PHP was built with debug.
    ///
    /// # Errors
    ///
    /// Returns an error if `PHPInfo` does not contain debug build information.
    pub fn debug(&self) -> Result<bool> {
        Ok(self
            .get_key("Debug Build")
            .context("Could not find debug build of PHP")?
            == "yes")
    }

    /// Get the PHP version string.
    ///
    /// # Errors
    ///
    /// Returns an error if `PHPInfo` does not contain version number.
    pub fn version(&self) -> Result<&str> {
        self.get_key("PHP Version")
            .context("Failed to get PHP version")
    }

    /// Get the Zend API version number.
    ///
    /// # Errors
    ///
    /// Returns an error if `PHPInfo` does not contain PHP API version.
    pub fn zend_version(&self) -> Result<u32> {
        self.get_key("PHP API")
            .context("Failed to get Zend version")
            .and_then(|s| u32::from_str(s).context("Failed to convert Zend version to integer"))
    }

    /// Get a key from the PHP info output.
    #[must_use]
    pub fn get_key(&self, key: &str) -> Option<&str> {
        let split = format!("{key} => ");
        for line in self.0.lines() {
            let components: Vec<_> = line.split(&split).collect();
            if components.len() > 1 {
                return Some(components[1]);
            }
        }
        None
    }

    /// Returns the raw PHP info output.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the PHP architecture (Windows only).
    ///
    /// # Errors
    ///
    /// Returns an error if `PHPInfo` does not contain architecture information.
    #[cfg(windows)]
    pub fn architecture(&self) -> Result<Arch> {
        self.get_key("Architecture")
            .context("Could not find architecture of PHP")?
            .try_into()
    }
}

/// PHP architecture (Windows only).
#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    /// 32-bit x86
    X86,
    /// 64-bit x86_64
    X64,
    /// 64-bit ARM
    AArch64,
}

#[cfg(windows)]
impl TryFrom<&str> for Arch {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "x86" => Ok(Self::X86),
            "x64" => Ok(Self::X64),
            "arm64" => Ok(Self::AArch64),
            arch => bail!("Unknown architecture: {}", arch),
        }
    }
}

#[cfg(windows)]
impl Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::X86 => write!(f, "x86"),
            Arch::X64 => write!(f, "x64"),
            Arch::AArch64 => write!(f, "arm64"),
        }
    }
}

/// PHP API version enum.
///
/// This enum represents the supported PHP API versions and provides utilities
/// for version detection and comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(clippy::inconsistent_digit_grouping)]
pub enum ApiVersion {
    /// PHP 8.0
    Php80 = 2020_09_30,
    /// PHP 8.1
    Php81 = 2021_09_02,
    /// PHP 8.2
    Php82 = 2022_08_29,
    /// PHP 8.3
    Php83 = 2023_08_31,
    /// PHP 8.4
    Php84 = 2024_09_24,
    /// PHP 8.5
    Php85 = 2025_09_25,
    /// PHP 8.6
    Php86 = 2025_09_26,
}

impl ApiVersion {
    /// Returns the maximum API version supported.
    #[must_use]
    pub const fn max() -> Self {
        ApiVersion::Php86
    }

    /// Returns all known API versions.
    #[must_use]
    pub fn versions() -> Vec<Self> {
        vec![
            ApiVersion::Php80,
            ApiVersion::Php81,
            ApiVersion::Php82,
            ApiVersion::Php83,
            ApiVersion::Php84,
            ApiVersion::Php85,
            ApiVersion::Php86,
        ]
    }

    /// Returns the API versions that are supported by this version.
    ///
    /// For example, PHP 8.3 supports APIs from 8.0, 8.1, 8.2, and 8.3.
    #[must_use]
    pub fn supported_apis(self) -> Vec<ApiVersion> {
        ApiVersion::versions()
            .into_iter()
            .filter(|&v| v <= self)
            .collect()
    }

    /// Returns the cfg flag name for this version (e.g., "php84").
    #[must_use]
    pub fn cfg_name(self) -> &'static str {
        match self {
            ApiVersion::Php80 => "php80",
            ApiVersion::Php81 => "php81",
            ApiVersion::Php82 => "php82",
            ApiVersion::Php83 => "php83",
            ApiVersion::Php84 => "php84",
            ApiVersion::Php85 => "php85",
            ApiVersion::Php86 => "php86",
        }
    }

    /// Returns the C preprocessor define name for this version.
    #[must_use]
    pub fn define_name(self) -> &'static str {
        match self {
            ApiVersion::Php80 => "EXT_PHP_RS_PHP_80",
            ApiVersion::Php81 => "EXT_PHP_RS_PHP_81",
            ApiVersion::Php82 => "EXT_PHP_RS_PHP_82",
            ApiVersion::Php83 => "EXT_PHP_RS_PHP_83",
            ApiVersion::Php84 => "EXT_PHP_RS_PHP_84",
            ApiVersion::Php85 => "EXT_PHP_RS_PHP_85",
            ApiVersion::Php86 => "EXT_PHP_RS_PHP_86",
        }
    }
}

impl TryFrom<u32> for ApiVersion {
    type Error = Error;

    fn try_from(version: u32) -> Result<Self, Self::Error> {
        match version {
            x if ((ApiVersion::Php80 as u32)..(ApiVersion::Php81 as u32)).contains(&x) => {
                Ok(ApiVersion::Php80)
            }
            x if ((ApiVersion::Php81 as u32)..(ApiVersion::Php82 as u32)).contains(&x) => {
                Ok(ApiVersion::Php81)
            }
            x if ((ApiVersion::Php82 as u32)..(ApiVersion::Php83 as u32)).contains(&x) => {
                Ok(ApiVersion::Php82)
            }
            x if ((ApiVersion::Php83 as u32)..(ApiVersion::Php84 as u32)).contains(&x) => {
                Ok(ApiVersion::Php83)
            }
            x if ((ApiVersion::Php84 as u32)..(ApiVersion::Php85 as u32)).contains(&x) => {
                Ok(ApiVersion::Php84)
            }
            x if ((ApiVersion::Php85 as u32)..(ApiVersion::Php86 as u32)).contains(&x) => {
                Ok(ApiVersion::Php85)
            }
            x if (ApiVersion::Php86 as u32) == x => Ok(ApiVersion::Php86),
            version => bail!(
                "The current version of PHP is not supported. Current PHP API version: {}, requires a version up to {}",
                version,
                ApiVersion::max() as u32
            ),
        }
    }
}

/// Emits cargo cfg flags for the detected PHP version.
///
/// This function prints `cargo:rustc-cfg=phpXX` for all supported API versions.
pub fn emit_php_cfg_flags(version: ApiVersion) {
    for supported_version in version.supported_apis() {
        println!("cargo:rustc-cfg={}", supported_version.cfg_name());
    }
}

/// Emits the cargo check-cfg directive for all PHP version flags.
///
/// Call this in your build script to avoid unknown cfg warnings.
pub fn emit_check_cfg() {
    println!(
        "cargo::rustc-check-cfg=cfg(php80, php81, php82, php83, php84, php85, php86, php_zts, php_debug, docs)"
    );
}

/// Emits cargo rerun-if-env-changed for PHP-related environment variables.
pub fn emit_rerun_if_env_changed() {
    println!("cargo:rerun-if-env-changed=PHP");
    println!("cargo:rerun-if-env-changed=PATH");
}
