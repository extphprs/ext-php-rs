use std::{path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};
use ext_php_rs_build::{find_executable, path_from_env};

use crate::{PHPInfo, PHPProvider};

pub struct Provider<'a> {
    info: &'a PHPInfo,
}

impl Provider<'_> {
    /// Runs `php-config` with one argument, returning the stdout.
    fn php_config(arg: &str) -> Result<String> {
        let cmd = Command::new(Self::find_bin()?)
            .arg(arg)
            .output()
            .context("Failed to run `php-config`")?;
        let stdout = String::from_utf8_lossy(&cmd.stdout);
        if !cmd.status.success() {
            let stderr = String::from_utf8_lossy(&cmd.stderr);
            bail!("Failed to run `php-config`: {stdout} {stderr}");
        }
        Ok(stdout.to_string())
    }

    fn find_bin() -> Result<PathBuf> {
        // If path is given via env, it takes priority.
        if let Some(path) = path_from_env("PHP_CONFIG") {
            if !path.try_exists()? {
                // If path was explicitly given and it can't be found, this is a hard error
                bail!("php-config executable not found at {}", path.display());
            }
            return Ok(path);
        }
        find_executable("php-config").with_context(|| {
            "Could not find `php-config` executable. \
            Please ensure `php-config` is in your PATH or the \
            `PHP_CONFIG` environment variable is set."
        })
    }
}

impl<'a> PHPProvider<'a> for Provider<'a> {
    fn new(info: &'a PHPInfo) -> Result<Self> {
        Ok(Self { info })
    }

    fn get_includes(&self) -> Result<Vec<PathBuf>> {
        Ok(Self::php_config("--includes")?
            .split(' ')
            .map(|s| s.trim_start_matches("-I"))
            .map(PathBuf::from)
            .collect())
    }

    fn get_defines(&self) -> Result<Vec<(&'static str, &'static str)>> {
        let mut defines = vec![];
        if self.info.thread_safety()? {
            defines.push(("ZTS", "1"));
        }
        Ok(defines)
    }

    fn print_extra_link_args(&self) -> Result<()> {
        // -lphp is opt-in: linking it into the production cdylib makes
        // ld.so map a second copy of libphp at extension load, and
        // function pointers like `zend_string_init_interned` read as NULL
        // from that copy. Tests on hosts that have libphp opt in via
        // EXT_PHP_RS_LINK_LIBPHP=1; the embed feature builds a standalone
        // binary that always needs it.
        //
        // Some PHP layouts ship the CLI but not a shared libphp at
        // <prefix>/lib (Homebrew `php@x.y` NTS, `php@x.y-debug-zts`). When
        // EXT_PHP_RS_LINK_LIBPHP=1 is set in those layouts we bail: relying on
        // `-Wl,-undefined,dynamic_lookup` to defer symbol resolution worked on
        // older macOS but not under chained fixups (default on macOS 13+ /
        // ld-prime), where missing data symbols abort the test binary at load.
        // The `embed` feature stays strict for the same reason.
        let force_link = std::env::var_os("EXT_PHP_RS_LINK_LIBPHP").is_some_and(|v| v == "1");
        if cfg!(feature = "embed") {
            let prefix = Self::php_config("--prefix")?.trim().to_owned();
            if !prefix.is_empty() {
                println!("cargo:rustc-link-search=native={prefix}/lib");
            }
            println!("cargo:rustc-link-lib=php");
        } else if force_link {
            let prefix = Self::php_config("--prefix")?.trim().to_owned();
            let lib_dir = (!prefix.is_empty()).then(|| format!("{prefix}/lib"));
            let libphp_exists = lib_dir
                .as_ref()
                .is_some_and(|dir| libphp_present_in(std::path::Path::new(dir)));
            if !libphp_exists {
                let searched_dir = lib_dir
                    .as_deref()
                    .unwrap_or("<empty `php-config --prefix`>");
                bail!(
                    "EXT_PHP_RS_LINK_LIBPHP=1 was set but no libphp shared library was found in \
                     {searched_dir}. Either install a libphp build at that path (for example via \
                     `libphp{{version}}-embed` on Debian/Ubuntu, or a PHP build configured with \
                     `--enable-embed`), point to it with `RUSTFLAGS=\"-L native=/path/to/libphp\"`, \
                     or unset EXT_PHP_RS_LINK_LIBPHP and skip the test step on this host."
                );
            }
            if let Some(dir) = lib_dir {
                println!("cargo:rustc-link-search=native={dir}");
            }
            println!("cargo:rustc-link-lib=php");
        }
        println!("cargo:rerun-if-env-changed=EXT_PHP_RS_LINK_LIBPHP");

        Ok(())
    }
}

fn libphp_present_in(dir: &std::path::Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with("libphp") {
            continue;
        }
        if has_shared_lib_extension(name) || name.contains(".so.") {
            return true;
        }
    }
    false
}

fn has_shared_lib_extension(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case("so")
                || ext.eq_ignore_ascii_case("dylib")
                || ext.eq_ignore_ascii_case("tbd")
                || ext.eq_ignore_ascii_case("a")
        })
}
