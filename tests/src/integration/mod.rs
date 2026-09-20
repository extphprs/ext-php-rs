pub mod array;
pub mod bailout;
pub mod binary;
pub mod binary_slice;
pub mod bool;
pub mod callable;
pub mod class;
pub mod closure;
pub mod defaults;
#[cfg(feature = "enum")]
pub mod enum_;
pub mod exception;
pub mod globals;
pub mod interface;
pub mod iterable;
pub mod iterator;
pub mod magic_method;
pub mod module_globals;
pub mod nullable;
pub mod number;
pub mod object;
#[cfg(feature = "observer")]
pub mod observer;
pub mod panic;
pub mod persistent_string;
pub mod reference;
pub mod separated;
pub mod string;
pub mod types;
pub mod variadic_args;

#[cfg(test)]
mod test {
    use std::env;

    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::Once;

    static BUILD: Once = Once::new();

    /// A `cargo build` for a workspace extension crate, with the feature set this
    /// test binary was compiled with. Every extension crate in `tests/` declares the
    /// same features, so `ext-php-rs` resolves to the artifact that is already built
    /// and its build script does not run again inside the test process.
    fn cargo_build(package: Option<&str>) -> Command {
        let mut command = Command::new("cargo");
        command.arg("build");
        if let Some(package) = package {
            command.args(["-p", package]);
        }

        #[cfg(not(debug_assertions))]
        command.arg("--release");

        // Build features list dynamically based on compiled features
        // Note: Using vec_init_then_push pattern here is intentional due to conditional
        // compilation
        #[allow(clippy::vec_init_then_push)]
        {
            let mut features = vec![];
            #[cfg(feature = "enum")]
            features.push("enum");
            #[cfg(feature = "closure")]
            features.push("closure");
            #[cfg(feature = "anyhow")]
            features.push("anyhow");
            #[cfg(feature = "runtime")]
            features.push("runtime");
            #[cfg(feature = "static")]
            features.push("static");
            #[cfg(feature = "observer")]
            features.push("observer");

            if !features.is_empty() {
                command.arg("--no-default-features");
                command.arg("--features").arg(features.join(","));
            }
        }
        command
    }

    fn setup() {
        BUILD.call_once(|| {
            let result = cargo_build(None)
                .output()
                .expect("failed to execute cargo build");

            assert!(
                result.status.success(),
                "Extension build failed:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        });
    }

    /// Finds the location of an executable `name`.
    pub fn find_executable(name: &str) -> Result<PathBuf, String> {
        const WHICH: &str = if cfg!(windows) { "where" } else { "which" };
        let cmd = Command::new(WHICH)
            .arg(name)
            .output()
            .map_err(|_| format!("Failed to execute \"{WHICH} {name}\""))?;
        if cmd.status.success() {
            let stdout = String::from_utf8(cmd.stdout)
                .map_err(|_| format!("Failed to parse output of \"{WHICH} {name}\""))?;

            stdout
                .trim()
                .lines()
                .next()
                .map(|l| l.trim().into())
                .ok_or_else(|| format!("No output from \"{WHICH} {name}\""))
        } else {
            Err(format!(
                "Executable \"{name}\" not found in PATH. \
                Please ensure it is installed and available in your PATH."
            ))
        }
    }

    /// Returns an environment variable's value as a `PathBuf`
    pub fn path_from_env(key: &str) -> Option<PathBuf> {
        std::env::var_os(key).map(PathBuf::from)
    }

    /// Finds the location of the PHP executable.
    pub fn find_php() -> Result<PathBuf, String> {
        // If path is given via env, it takes priority.
        if let Some(path) = path_from_env("PHP") {
            if !path
                .try_exists()
                .map_err(|e| format!("Could not check existence: {e}"))?
            {
                // If path was explicitly given and it can't be found, this is a hard error
                return Err(format!("php executable not found at {path:?}"));
            }
            return Ok(path);
        }
        find_executable("php").map_err(|_| {
            "Could not find PHP executable. \
            Please ensure `php` is in your PATH or the `PHP` environment variable is set."
                .into()
        })
    }

    /// Gets the path to the compiled test extension.
    pub fn get_extension_path() -> String {
        let mut path = env::current_dir().expect("Could not get cwd");
        path.pop();
        path.push("target");

        #[cfg(not(debug_assertions))]
        path.push("release");
        #[cfg(debug_assertions)]
        path.push("debug");

        path.push(if std::env::consts::DLL_EXTENSION == "dll" {
            "tests"
        } else {
            "libtests"
        });
        path.set_extension(std::env::consts::DLL_EXTENSION);
        path.to_str().unwrap().to_string()
    }

    #[cfg(feature = "embed")]
    pub fn run_php_embed(file: &str) {
        use ext_php_rs::embed::Embed;
        use ext_php_rs::ffi::zend_register_module_ex;
        use std::sync::OnceLock;

        // get_module() can only be called once because observer registration
        // uses OnceLock internally. Cache the pointer for reuse across tests.
        struct ModulePtr(*mut ext_php_rs::ffi::zend_module_entry);
        unsafe impl Send for ModulePtr {}
        unsafe impl Sync for ModulePtr {}
        static MODULE: OnceLock<ModulePtr> = OnceLock::new();

        let module = MODULE.get_or_init(|| ModulePtr(crate::get_module())).0;

        Embed::run(|| {
            #[cfg(php84)]
            {
                unsafe { zend_register_module_ex(module, 2) };
                ext_php_rs::zend::ExecutorGlobals::get_mut().full_tables_cleanup = true;
            }
            #[cfg(not(php84))]
            unsafe {
                zend_register_module_ex(module)
            };

            Embed::eval("ini_set('assert.exception', '1');")
                .expect("Failed to set assert.exception");

            let result = Embed::run_script(format!("src/integration/{file}"));
            assert!(result.is_ok(), "PHP script {file} failed: {result:?}");
        });
    }

    /// Builds the named broken extension crate and loads it in a `php`
    /// subprocess. Returns the exit status and the combined stdout and stderr.
    pub fn load_broken_module(crate_name: &str) -> (std::process::ExitStatus, String) {
        let built = cargo_build(Some(crate_name))
            .output()
            .expect("failed to execute cargo build");
        assert!(
            built.status.success(),
            "{crate_name} build failed:\n{}",
            String::from_utf8_lossy(&built.stderr)
        );

        let lib_name = crate_name.replace('-', "_");
        let mut path = PathBuf::from(get_extension_path());
        path.set_file_name(if std::env::consts::DLL_EXTENSION == "dll" {
            lib_name
        } else {
            format!("lib{lib_name}")
        });
        path.set_extension(std::env::consts::DLL_EXTENSION);

        let output = Command::new(find_php().expect("Could not find PHP executable"))
            .arg(format!("-dextension={}", path.display()))
            .arg("-ddisplay_startup_errors=1")
            .args(["-r", "echo 'alive';"])
            .output()
            .expect("failed to run php");
        let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
        (output.status, combined)
    }

    pub fn run_php(file: &str) -> bool {
        run_php_capturing_stderr(file);
        true
    }

    /// Runs the script in a real `php` subprocess, panics unless it exits
    /// successfully, and returns what it wrote to stderr.
    pub fn run_php_capturing_stderr(file: &str) -> String {
        setup();
        let path = get_extension_path();
        let output = Command::new(find_php().expect("Could not find PHP executable"))
            .arg(format!("-dextension={path}"))
            .arg("-dassert.active=1")
            .arg("-dassert.exception=1")
            .arg("-dzend.assertions=1")
            .arg(format!("src/integration/{file}"))
            .output()
            .expect("failed to run php file");
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(
            output.status.success(),
            "
                status: {}
                stdout: {}
                stderr: {}
                ",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            stderr
        );
        stderr
    }
}
