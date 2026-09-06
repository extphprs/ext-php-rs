use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use ext_php_rs_introspection::Description;
use libloading::os::unix::{Library, RTLD_LAZY, RTLD_LOCAL, Symbol};

pub struct Ext {
    // These need to be here to keep the libraries alive. The extension library needs to be alive
    // to access the describe function. Missing here is the lifetime on `Symbol<'a, fn() ->
    // Module>` where `ext_lib: 'a`.
    #[allow(dead_code)]
    ext_lib: Library,
    describe_fn: Symbol<extern "C" fn() -> *mut Description>,
}

impl Ext {
    /// Loads an extension.
    pub fn load(ext_path: PathBuf) -> Result<Self> {
        // On macOS, add RTLD_FIRST for two-level namespace which properly defers
        // resolution of symbols not needed for stub generation.
        #[cfg(target_os = "macos")]
        let ext_lib =
            unsafe { Library::open(Some(ext_path), RTLD_LAZY | RTLD_LOCAL | libc::RTLD_FIRST) }
                .with_context(|| "Failed to load extension library")?;

        // On other Unix platforms, RTLD_LAZY | RTLD_LOCAL is sufficient
        #[cfg(not(target_os = "macos"))]
        let ext_lib = unsafe { Library::open(Some(ext_path), RTLD_LAZY | RTLD_LOCAL) }
            .with_context(|| "Failed to load extension library")?;

        // The v1 symbol returned `Description` by value, a different calling
        // convention. Calling it through the v2 signature would corrupt
        // memory, so an extension that only exports v1 is rejected up front.
        let describe_fn = match unsafe { ext_lib.get(b"ext_php_rs_describe_module_v2") } {
            Ok(describe_fn) => describe_fn,
            Err(err) => {
                if unsafe { ext_lib.get::<*const ()>(b"ext_php_rs_describe_module") }.is_ok() {
                    bail!(
                        "Extension was built with ext-php-rs 0.15 or older, whose describe entry point is incompatible with this `cargo-php`. Rebuild the extension against ext-php-rs 0.16 or use `cargo install cargo-php --version 0.1`."
                    );
                }
                return Err(err)
                    .context("Failed to load describe function symbol from extension library");
            }
        };

        Ok(Self {
            ext_lib,
            describe_fn,
        })
    }

    /// Calls the extension's describe entry point.
    ///
    /// The extension allocates the [`Description`] on its heap and returns a
    /// pointer, so the CLI's stack never holds a value whose size the
    /// extension decided. Only `Description::version`, the first field, may
    /// be read before the version check passes; the caller frees the value
    /// with [`Box::from_raw`] once the layout is known to match.
    pub fn describe(&self) -> *mut Description {
        (self.describe_fn)()
    }
}
