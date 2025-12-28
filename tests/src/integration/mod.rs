pub mod array;
pub mod bailout;
pub mod binary;
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
pub mod iterator;
pub mod magic_method;
pub mod nullable;
pub mod number;
pub mod object;
pub mod persistent_string;
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

    fn setup() {
        BUILD.call_once(|| {
            let mut command = Command::new("cargo");
            command.arg("build");

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

                if !features.is_empty() {
                    command.arg("--no-default-features");
                    command.arg("--features").arg(features.join(","));
                }
            }

            let result = command.output().expect("failed to execute cargo build");

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

    pub fn run_php(file: &str) -> bool {
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
        if output.status.success() {
            true
        } else {
            panic!(
                "
                status: {}
                stdout: {}
                stderr: {}
                ",
                output.status,
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            );
        }
    }
}
