//! The build script for ext-php-rs.
//! This script is responsible for generating the bindings to the PHP Zend API.
//! It also checks the PHP version for compatibility with ext-php-rs and sets
//! configuration flags accordingly.
#[cfg_attr(windows, path = "windows_build.rs")]
#[cfg_attr(not(windows), path = "unix_build.rs")]
mod impl_;

use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::{Context, Result, anyhow};
use bindgen::RustTarget;
use ext_php_rs_build::{ApiVersion, PHPInfo, find_php};
use impl_::Provider;

/// Provides information about the PHP installation.
pub trait PHPProvider<'a>: Sized {
    /// Create a new PHP provider.
    #[allow(clippy::missing_errors_doc)]
    fn new(info: &'a PHPInfo) -> Result<Self>;

    /// Retrieve a list of absolute include paths.
    #[allow(clippy::missing_errors_doc)]
    fn get_includes(&self) -> Result<Vec<PathBuf>>;

    /// Retrieve a list of macro definitions to pass to the compiler.
    #[allow(clippy::missing_errors_doc)]
    fn get_defines(&self) -> Result<Vec<(&'static str, &'static str)>>;

    /// Writes the bindings to a file.
    #[allow(clippy::missing_errors_doc)]
    fn write_bindings(&self, bindings: String, writer: &mut impl Write) -> Result<()> {
        for line in bindings.lines() {
            writeln!(writer, "{line}")?;
        }
        Ok(())
    }

    /// Prints any extra link arguments.
    #[allow(clippy::missing_errors_doc)]
    fn print_extra_link_args(&self) -> Result<()> {
        Ok(())
    }
}

fn add_php_version_defines(
    defines: &mut Vec<(&'static str, &'static str)>,
    info: &PHPInfo,
) -> Result<()> {
    let version = info.zend_version()?;
    let supported_version: ApiVersion = version.try_into()?;

    for supported_api in supported_version.supported_apis() {
        defines.push((supported_api.define_name(), "1"));
    }

    Ok(())
}

/// Builds the wrapper library.
fn build_wrapper(defines: &[(&str, &str)], includes: &[PathBuf]) -> Result<()> {
    let mut build = cc::Build::new();
    for (var, val) in defines {
        build.define(var, *val);
    }
    build
        .file("src/wrapper.c")
        .includes(includes)
        .try_compile("wrapper")
        .context("Failed to compile ext-php-rs C interface")?;
    Ok(())
}

#[cfg(feature = "embed")]
/// Builds the embed library.
fn build_embed(defines: &[(&str, &str)], includes: &[PathBuf]) -> Result<()> {
    let mut build = cc::Build::new();
    for (var, val) in defines {
        build.define(var, *val);
    }
    build
        .file("src/embed/embed.c")
        .includes(includes)
        .try_compile("embed")
        .context("Failed to compile ext-php-rs C embed interface")?;
    Ok(())
}

/// Generates bindings to the Zend API.
fn generate_bindings(defines: &[(&str, &str)], includes: &[PathBuf]) -> Result<String> {
    let mut bindgen = bindgen::Builder::default();

    #[cfg(feature = "embed")]
    {
        bindgen = bindgen.header("src/embed/embed.h");
    }

    bindgen = bindgen
        .header("src/wrapper.h")
        .clang_args(
            includes
                .iter()
                .map(|inc| format!("-I{}", inc.to_string_lossy())),
        )
        .clang_args(defines.iter().map(|(var, val)| format!("-D{var}={val}")));

    // Add macOS SDK path for system headers (stdlib.h, etc.)
    // Required for libclang 19+ with preserve_none calling convention support
    #[cfg(target_os = "macos")]
    if let Some(sdk_path) = std::process::Command::new("xcrun")
        .args(["--show-sdk-path"])
        .output()
        .ok()
        .filter(|output| output.status.success())
    {
        let path = String::from_utf8_lossy(&sdk_path.stdout);
        let path = path.trim();
        bindgen = bindgen
            .clang_arg(format!("-isysroot{path}"))
            .clang_arg(format!("-I{path}/usr/include"));
    }

    bindgen = bindgen
        .formatter(bindgen::Formatter::Rustfmt)
        .no_copy("php_ini_builder")
        .no_copy("_zval_struct")
        .no_copy("_zend_string")
        .no_copy("_zend_array")
        .no_debug("_zend_function_entry") // On Windows when the handler uses vectorcall, Debug cannot be derived so we do it in code.
        .layout_tests(env::var("EXT_PHP_RS_TEST").is_ok())
        .rust_target(RustTarget::nightly());

    for binding in ALLOWED_BINDINGS {
        bindgen = bindgen
            .allowlist_function(binding)
            .allowlist_type(binding)
            .allowlist_var(binding);
    }

    let extension_allowed_bindings = env::var("EXT_PHP_RS_ALLOWED_BINDINGS").ok();
    if let Some(extension_allowed_bindings) = extension_allowed_bindings {
        for binding in extension_allowed_bindings.split(',') {
            bindgen = bindgen
                .allowlist_function(binding)
                .allowlist_type(binding)
                .allowlist_var(binding);
        }
    }

    let bindings = bindgen
        .generate()
        .map_err(|_| anyhow!("Unable to generate bindings for PHP"))?
        .to_string();

    Ok(bindings)
}

/// Returns the minimum API version supported by ext-php-rs.
fn min_api_version() -> ApiVersion {
    [
        ApiVersion::Php80,
        #[cfg(feature = "enum")]
        ApiVersion::Php81,
    ]
    .into_iter()
    .max()
    .unwrap_or(ApiVersion::max())
}

/// Checks the PHP Zend API version for compatibility with ext-php-rs, setting
/// any configuration flags required.
fn check_php_version(info: &PHPInfo) -> Result<()> {
    let version = info.zend_version()?;
    let version: ApiVersion = version.try_into()?;

    // Infra cfg flags - use these for things that change in the Zend API that don't
    // rely on a feature and the crate user won't care about (e.g. struct field
    // changes). Use a feature flag for an actual feature (e.g. enums being
    // introduced in PHP 8.1).
    //
    // PHP 8.0 is the baseline - no feature flags will be introduced here.
    //
    // The PHP version cfg flags should also stack - if you compile on PHP 8.2 you
    // should get both the `php81` and `php82` flags.
    ext_php_rs_build::emit_check_cfg();

    if version == ApiVersion::Php80 {
        println!(
            "cargo:warning=PHP 8.0 is EOL and is no longer supported. Please upgrade to a supported version of PHP. See https://www.php.net/supported-versions.php for information on version support timelines."
        );
    }

    if version < min_api_version() {
        anyhow::bail!(
            "PHP version {} is below minimum supported version",
            version.cfg_name()
        );
    }

    ext_php_rs_build::emit_php_cfg_flags(version);

    Ok(())
}

fn main() -> Result<()> {
    let out_dir = env::var_os("OUT_DIR").context("Failed to get OUT_DIR")?;
    let out_path = PathBuf::from(out_dir).join("bindings.rs");
    let manifest: PathBuf = std::env::var("CARGO_MANIFEST_DIR").unwrap().into();
    for path in [
        manifest.join("src").join("wrapper.h"),
        manifest.join("src").join("wrapper.c"),
        manifest.join("src").join("embed").join("embed.h"),
        manifest.join("src").join("embed").join("embed.c"),
        manifest.join("allowed_bindings.rs"),
        manifest.join("windows_build.rs"),
        manifest.join("unix_build.rs"),
    ] {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
    for env_var in ["PHP", "PHP_CONFIG", "PATH", "EXT_PHP_RS_ALLOWED_BINDINGS"] {
        println!("cargo:rerun-if-env-changed={env_var}");
    }

    println!("cargo:rerun-if-changed=build.rs");

    // docs.rs runners only have PHP 7.4 - use pre-generated bindings
    if env::var("DOCS_RS").is_ok() {
        println!("cargo:warning=docs.rs detected - using stub bindings");
        println!("cargo:rustc-cfg=php_debug");
        println!("cargo:rustc-cfg=php81");
        println!("cargo:rustc-cfg=php82");
        println!("cargo:rustc-cfg=php83");
        println!("cargo:rustc-cfg=php84");
        println!("cargo:rustc-cfg=php85");
        std::fs::copy("docsrs_bindings.rs", out_path)
            .expect("failed to copy docs.rs stub bindings to out directory");
        return Ok(());
    }

    let php = find_php()?;
    let info = PHPInfo::get(&php)?;
    let provider = Provider::new(&info)?;

    let includes = provider.get_includes()?;
    let mut defines = provider.get_defines()?;
    add_php_version_defines(&mut defines, &info)?;

    check_php_version(&info)?;
    build_wrapper(&defines, &includes)?;

    #[cfg(feature = "embed")]
    build_embed(&defines, &includes)?;

    let bindings = generate_bindings(&defines, &includes)?;

    let out_file =
        File::create(&out_path).context("Failed to open output bindings file for writing")?;
    let mut out_writer = BufWriter::new(out_file);
    provider.write_bindings(bindings, &mut out_writer)?;

    if info.debug()? {
        println!("cargo:rustc-cfg=php_debug");
    }
    if info.thread_safety()? {
        println!("cargo:rustc-cfg=php_zts");
    }
    provider.print_extra_link_args()?;

    // Generate guide tests
    let test_md = skeptic::markdown_files_of_directory("guide");
    #[cfg(not(feature = "closure"))]
    let test_md: Vec<_> = test_md
        .into_iter()
        .filter(|p| p.file_stem() != Some(std::ffi::OsStr::new("closure")))
        .collect();
    skeptic::generate_doc_tests(&test_md);

    Ok(())
}

// Mock macro for the `allowed_bindings.rs` script.
macro_rules! bind {
    ($($s: ident),*) => {
        &[$(
            stringify!($s),
        )*]
    }
}

/// Array of functions/types used in `ext-php-rs` - used to allowlist when
/// generating bindings, as we don't want to generate bindings for everything
/// (i.e. stdlib headers).
const ALLOWED_BINDINGS: &[&str] = include!("allowed_bindings.rs");
