use std::path::PathBuf;

use anyhow::{Context, Result};
use ext_php_rs::describe::Description;
use libloading::os::unix::{Library, RTLD_LAZY, RTLD_LOCAL, Symbol};

#[allow(improper_ctypes_definitions)]
pub struct Ext {
    // These need to be here to keep the libraries alive. The extension library needs to be alive
    // to access the describe function. Missing here is the lifetime on `Symbol<'a, fn() ->
    // Module>` where `ext_lib: 'a`.
    #[allow(dead_code)]
    ext_lib: Library,
    describe_fn: Symbol<extern "C" fn() -> Description>,
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

        let describe_fn = unsafe {
            ext_lib
                .get(b"ext_php_rs_describe_module")
                .with_context(|| "Failed to load describe function symbol from extension library")?
        };

        Ok(Self {
            ext_lib,
            describe_fn,
        })
    }

    /// Describes the extension.
    pub fn describe(&self) -> Description {
        (self.describe_fn)()
    }
}
