use anyhow::{Context, Result, bail};
use std::{
    convert::TryFrom,
    fmt::Display,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};
use ureq::tls::{TlsConfig, TlsProvider};

use crate::{PHPInfo, PHPProvider};

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub struct Provider<'a> {
    info: &'a PHPInfo,
    devel: DevelPack,
}

impl<'a> Provider<'a> {
    /// Retrieves the PHP library name (filename without extension).
    fn get_php_lib_name(&self) -> Result<String> {
        Ok(self
            .devel
            .php_lib(self.info.debug()?)
            .file_stem()
            .context("Failed to get PHP library name")?
            .to_string_lossy()
            .to_string())
    }
}

impl<'a> PHPProvider<'a> for Provider<'a> {
    fn new(info: &'a PHPInfo) -> Result<Self> {
        let version = info.version()?;
        let is_zts = info.thread_safety()?;
        let arch = info.architecture()?;
        let devel = DevelPack::new(version, is_zts, arch)?;
        if let Ok(linker) = get_rustc_linker() {
            if looks_like_msvc_linker(&linker) {
                println!(
                    "cargo:warning=It looks like you are using a MSVC linker. You may encounter issues when attempting to load your compiled extension into PHP if your MSVC linker version is not compatible with the linker used to compile your PHP. It is recommended to use `rust-lld` as your linker."
                );
            }
        }

        Ok(Self { info, devel })
    }

    fn get_includes(&self) -> Result<Vec<PathBuf>> {
        Ok(self.devel.include_paths())
    }

    fn get_defines(&self) -> Result<Vec<(&'static str, &'static str)>> {
        let mut defines = vec![
            ("ZEND_WIN32", "1"),
            ("PHP_WIN32", "1"),
            ("WINDOWS", "1"),
            ("WIN32", "1"),
            ("ZEND_DEBUG", if self.info.debug()? { "1" } else { "0" }),
        ];
        if self.info.thread_safety()? {
            defines.push(("ZTS", "1"));
        }
        Ok(defines)
    }

    fn write_bindings(&self, bindings: String, writer: &mut impl Write) -> Result<()> {
        // For some reason some symbols don't link without a `#[link(name = "php8")]`
        // attribute on each extern block. Bindgen doesn't give us the option to add
        // this so we need to add it manually.
        //
        // Note: bindgen 0.72+ generates `unsafe extern` blocks when using nightly Rust,
        // so we need to handle both `extern` and `unsafe extern` variants.
        let php_lib_name = self.get_php_lib_name()?;
        for line in bindings.lines() {
            match line {
                "extern \"C\" {"
                | "extern \"fastcall\" {"
                | "unsafe extern \"C\" {"
                | "unsafe extern \"fastcall\" {" => {
                    writeln!(writer, "#[link(name = \"{}\")]", php_lib_name)?;
                }
                _ => {}
            }
            writeln!(writer, "{}", line)?;
        }
        Ok(())
    }

    fn print_extra_link_args(&self) -> Result<()> {
        let php_lib_name = self.get_php_lib_name()?;
        let php_lib_search = self
            .devel
            .php_lib(self.info.debug()?)
            .parent()
            .context("Failed to get PHP library parent folder")?
            .to_string_lossy()
            .to_string();
        println!("cargo:rustc-link-lib=dylib={}", php_lib_name);
        println!("cargo:rustc-link-search={}", php_lib_search);
        Ok(())
    }
}

/// Returns the path to rustc's linker.
fn get_rustc_linker() -> Result<PathBuf> {
    // `RUSTC_LINKER` is set if the linker has been overridden anywhere.
    if let Ok(link) = std::env::var("RUSTC_LINKER") {
        return Ok(link.into());
    }

    let link = cc::windows_registry::find_tool(
        &std::env::var("TARGET").context("`TARGET` environment variable not set")?,
        "link.exe",
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to retrieve linker tool from Windows registry"))?;
    Ok(link.path().to_owned())
}

/// Checks if a linker looks like the MSVC link.exe linker.
fn looks_like_msvc_linker(linker: &Path) -> bool {
    let command = Command::new(linker).output();
    if let Ok(command) = command {
        let stdout = String::from_utf8_lossy(&command.stdout);
        if stdout.contains("Microsoft (R) Incremental Linker") {
            return true;
        }
    }
    false
}

#[derive(Debug, PartialEq, Eq)]
pub enum Arch {
    X86,
    X64,
    AArch64,
}

impl TryFrom<&str> for Arch {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        Ok(match value {
            "x86" => Self::X86,
            "x64" => Self::X64,
            "arm64" => Self::AArch64,
            a => bail!("Unknown architecture {}", a),
        })
    }
}

impl Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Arch::X86 => "x86",
                Arch::X64 => "x64",
                Arch::AArch64 => "arm64",
            }
        )
    }
}

struct DevelPack(PathBuf);

impl DevelPack {
    /// Downloads a new PHP development pack, unzips it in the build script
    /// temporary directory.
    fn new(version: &str, is_zts: bool, arch: Arch) -> Result<DevelPack> {
        // If the PHP version is more than 8.4.1, use VS17 instead of VS16.
        let version_float = version
            .split('.')
            .take(2)
            .collect::<Vec<_>>()
            .join(".")
            .parse::<f32>()
            .context("Failed to parse PHP version as float")?;

        // PHP builds switched to VS17 in PHP 8.4.1.
        let visual_studio_version = if version_float >= 8.4f32 {
            "vs17"
        } else {
            "vs16"
        };

        let zip_name = format!(
            "php-devel-pack-{}{}-Win32-{}-{}.zip",
            version,
            if is_zts { "" } else { "-nts" },
            visual_studio_version,
            arch
        );

        fn download(zip_name: &str, archive: bool) -> Result<PathBuf> {
            let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
            let url = format!(
                "https://windows.php.net/downloads/releases{}/{}",
                if archive { "/archives" } else { "" },
                zip_name
            );
            let response = ureq::Agent::config_builder()
                .tls_config(
                    TlsConfig::builder()
                        .provider(TlsProvider::NativeTls)
                        .build(),
                )
                .build()
                .new_agent()
                .get(&url)
                .header("User-Agent", USER_AGENT)
                .call()
                .context("Failed to download development pack")?;
            let mut content = vec![];
            response
                .into_body()
                .into_reader()
                .read_to_end(&mut content)
                .context("Failed to read development pack")?;
            let mut content = Cursor::new(&mut content);
            let mut zip_content = zip::read::ZipArchive::new(&mut content)
                .context("Failed to unzip development pack")?;
            let inner_name = zip_content
                .file_names()
                .next()
                .and_then(|f| f.split('/').next())
                .context("Failed to get development pack name")?;
            let devpack_path = out_dir.join(inner_name);
            let _ = std::fs::remove_dir_all(&devpack_path);
            zip_content
                .extract(&out_dir)
                .context("Failed to extract devpack to directory")?;
            Ok(devpack_path)
        }

        download(&zip_name, false)
            .or_else(|_| download(&zip_name, true))
            .map(DevelPack)
    }

    /// Returns the path to the include folder.
    pub fn includes(&self) -> PathBuf {
        self.0.join("include")
    }

    /// Returns the path of the PHP library containing symbols for linking.
    pub fn php_lib(&self, is_debug: bool) -> PathBuf {
        let php_lib_path = std::env::var("PHP_LIB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.0.join("lib"));

        if !php_lib_path.exists() {
            panic!(
                "Error: Specified PHP library path '{}' does not exist.",
                php_lib_path.display()
            );
        }

        let candidates = if is_debug {
            ["php8_debug.lib", "php8ts_debug.lib"]
        } else {
            ["php8.lib", "php8ts.lib"]
        };

        candidates
            .iter()
            .map(|lib| php_lib_path.join(lib))
            .find(|path| path.exists())
            .expect(&format!(
                "{}",
                if is_debug {
                    format!(
                        r#"Error: No suitable PHP library found in '{}'.
To build the application in DEBUG mode on Windows,
you must have a PHP SDK built with the DEBUG option enabled
and specify the PHP_LIB to the folder containing the lib files.
For example: set PHP_LIB=C:\php-sdk\php-dev\vc16\x64\php-8.3.13-src\x64\Debug_TS."#,
                        php_lib_path.display()
                    )
                } else {
                    format!(
                        "Error: No suitable PHP library found in '{}'.",
                        php_lib_path.display()
                    )
                }
            ))
    }

    /// Returns a list of include paths to pass to the compiler.
    pub fn include_paths(&self) -> Vec<PathBuf> {
        let includes = self.includes();
        ["", "main", "Zend", "TSRM", "ext"]
            .iter()
            .map(|p| includes.join(p))
            .collect()
    }
}
