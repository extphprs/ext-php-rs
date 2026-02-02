#![doc = include_str!("../README.md")]

#[cfg(not(windows))]
mod ext;

use anyhow::{Context, Result as AResult, bail};
use cargo_metadata::{CrateType, Target, camino::Utf8PathBuf};
use clap::Parser;
use dialoguer::{Confirm, Select};

use std::{
    env::consts,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

/// Generates mock symbols required to generate stub files from a downstream
/// crates CLI application.
#[macro_export]
macro_rules! stub_symbols {
    ($($s: ident),*) => {
        $(
            $crate::stub_symbols!(@INTERNAL; $s);
        )*
    };
    (@INTERNAL; $s: ident) => {
        #[allow(non_upper_case_globals)]
        #[allow(missing_docs)]
        #[unsafe(no_mangle)]
        pub static mut $s: *mut () = ::std::ptr::null_mut();
    };
}

/// Result type returned from the [`run`] function.
pub type CrateResult = AResult<()>;

/// Runs the CLI application. Returns nothing in a result on success.
///
/// # Errors
///
/// Returns an error if the application fails to run.
pub fn run() -> CrateResult {
    let mut args: Vec<_> = std::env::args().collect();

    // When called as a cargo subcommand, the second argument given will be the
    // subcommand, in this case `php`. We don't want this so we remove from args and
    // pass it to clap.
    if args.get(1).is_some_and(|nth| nth == "php") {
        args.remove(1);
    }

    Args::parse_from(args).handle()
}

#[derive(Parser)]
#[clap(
    about = "Installs extensions and generates stub files for PHP extensions generated with `ext-php-rs`.",
    author = "David Cole <david.cole1340@gmail.com>",
    version = env!("CARGO_PKG_VERSION")
)]
enum Args {
    /// Installs the extension in the current PHP installation.
    ///
    /// This copies the extension to the PHP installation and adds the
    /// extension to a PHP configuration file.
    ///
    /// Note that this uses the `php-config` executable installed alongside PHP
    /// to locate your `php.ini` file and extension directory. If you want to
    /// use a different `php-config`, the application will read the `PHP_CONFIG`
    /// variable (if it is set), and will use this as the path to the executable
    /// instead.
    Install(Install),
    /// Removes the extension in the current PHP installation.
    ///
    /// This deletes the extension from the PHP installation and also removes it
    /// from the main PHP configuration file.
    ///
    /// Note that this uses the `php-config` executable installed alongside PHP
    /// to locate your `php.ini` file and extension directory. If you want to
    /// use a different `php-config`, the application will read the `PHP_CONFIG`
    /// variable (if it is set), and will use this as the path to the executable
    /// instead.
    Remove(Remove),
    /// Generates stub PHP files for the extension.
    ///
    /// These stub files can be used in IDEs to provide typehinting for
    /// extension classes, functions and constants.
    #[cfg(not(windows))]
    Stubs(Stubs),
    /// Watches for changes and automatically rebuilds and installs the extension.
    ///
    /// This command watches Rust source files and Cargo.toml for changes,
    /// automatically rebuilding and reinstalling the extension when changes
    /// are detected. Optionally, it can also manage the PHP built-in development
    /// server, restarting it after each successful rebuild.
    #[cfg(not(windows))]
    Watch(Watch),
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Parser)]
struct Install {
    /// Changes the path that the extension is copied to. This will not
    /// activate the extension unless `ini_path` is also passed.
    #[arg(long)]
    #[allow(clippy::struct_field_names)]
    install_dir: Option<PathBuf>,
    /// Path to the `php.ini` file to update with the new extension.
    #[arg(long)]
    ini_path: Option<PathBuf>,
    /// Installs the extension but doesn't enable the extension in the `php.ini`
    /// file.
    #[arg(long)]
    disable: bool,
    /// Whether to install the release version of the extension.
    #[arg(long)]
    release: bool,
    /// Path to the Cargo manifest of the extension. Defaults to the manifest in
    /// the directory the command is called.
    #[arg(long)]
    manifest: Option<PathBuf>,
    #[arg(short = 'F', long, num_args = 1..)]
    features: Option<Vec<String>>,
    #[arg(long)]
    all_features: bool,
    #[arg(long)]
    no_default_features: bool,
    /// Whether to bypass the install prompt.
    #[clap(long)]
    yes: bool,
    /// Skip the smoke test that verifies the extension loads correctly.
    #[clap(long)]
    no_smoke_test: bool,
}

#[derive(Parser)]
struct Remove {
    /// Changes the path that the extension will be removed from. This will not
    /// remove the extension from a configuration file unless `ini_path` is also
    /// passed.
    #[arg(long)]
    install_dir: Option<PathBuf>,
    /// Path to the `php.ini` file to remove the extension from.
    #[arg(long)]
    ini_path: Option<PathBuf>,
    /// Path to the Cargo manifest of the extension. Defaults to the manifest in
    /// the directory the command is called.
    #[arg(long)]
    manifest: Option<PathBuf>,
    /// Whether to bypass the remove prompt.
    #[clap(long)]
    yes: bool,
}

#[cfg(not(windows))]
#[derive(Parser)]
struct Stubs {
    /// Path to extension to generate stubs for. Defaults for searching the
    /// directory the executable is located in.
    ext: Option<PathBuf>,
    /// Path used to store generated stub file. Defaults to writing to
    /// `<ext-name>.stubs.php` in the current directory.
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Print stubs to stdout rather than write to file. Cannot be used with
    /// `out`.
    #[arg(long, conflicts_with = "out")]
    stdout: bool,
    /// Path to the Cargo manifest of the extension. Defaults to the manifest in
    /// the directory the command is called.
    ///
    /// This cannot be provided alongside the `ext` option, as that option
    /// provides a direct path to the extension shared library.
    #[arg(long, conflicts_with = "ext")]
    manifest: Option<PathBuf>,
    #[arg(short = 'F', long, num_args = 1..)]
    features: Option<Vec<String>>,
    #[arg(long)]
    all_features: bool,
    #[arg(long)]
    no_default_features: bool,
}

#[cfg(not(windows))]
#[allow(clippy::struct_excessive_bools)]
#[derive(Parser)]
struct Watch {
    /// Command to run after each build (e.g., 'php test.php').
    /// The command is executed via shell and restarted on each rebuild.
    #[arg(trailing_var_arg = true)]
    command: Vec<String>,
    /// Start PHP built-in server and restart it on changes.
    #[arg(long)]
    serve: bool,
    /// Host and port for PHP server (e.g., localhost:8000).
    #[arg(long, default_value = "localhost:8000")]
    host: String,
    /// Document root for PHP server. Defaults to current directory.
    #[arg(long)]
    docroot: Option<PathBuf>,
    /// Whether to build the release version of the extension.
    #[arg(long)]
    release: bool,
    /// Path to the Cargo manifest of the extension. Defaults to the manifest in
    /// the directory the command is called.
    #[arg(long)]
    manifest: Option<PathBuf>,
    #[arg(short = 'F', long, num_args = 1..)]
    features: Option<Vec<String>>,
    #[arg(long)]
    all_features: bool,
    #[arg(long)]
    no_default_features: bool,
    /// Changes the path that the extension is copied to.
    #[arg(long)]
    install_dir: Option<PathBuf>,
}

impl Args {
    pub fn handle(self) -> CrateResult {
        match self {
            Args::Install(install) => install.handle(),
            Args::Remove(remove) => remove.handle(),
            #[cfg(not(windows))]
            Args::Stubs(stubs) => stubs.handle(),
            #[cfg(not(windows))]
            Args::Watch(watch) => watch.handle(),
        }
    }
}

impl Install {
    #[allow(clippy::too_many_lines)]
    pub fn handle(self) -> CrateResult {
        let artifact = find_ext(self.manifest.as_ref())?;
        let ext_path = build_ext(
            &artifact,
            self.release,
            self.features,
            self.all_features,
            self.no_default_features,
        )?;

        let (mut ext_dir, mut php_ini) = if let Some(install_dir) = self.install_dir {
            (install_dir, None)
        } else {
            (get_ext_dir()?, Some(get_php_ini()?))
        };

        if let Some(ini_path) = self.ini_path {
            php_ini = Some(ini_path);
        }

        if !self.yes
            && !Confirm::new()
                .with_prompt(format!(
                    "Are you sure you want to install the extension `{}`?",
                    artifact.name
                ))
                .interact()?
        {
            bail!("Installation cancelled.");
        }

        debug_assert!(ext_path.is_file());
        let ext_name = ext_path.file_name().expect("ext path wasn't a filepath");

        if ext_dir.is_dir() {
            ext_dir.push(ext_name);
        }

        // Use atomic copy: copy to temp file in same directory, then rename.
        // This prevents race conditions where a partially-written extension could be loaded.
        let temp_ext_path = ext_dir.with_extension(format!(
            "{}.tmp.{}",
            ext_dir
                .extension()
                .map(|e| e.to_string_lossy())
                .unwrap_or_default(),
            std::process::id()
        ));

        std::fs::copy(&ext_path, &temp_ext_path).with_context(
            || "Failed to copy extension from target directory to extension directory",
        )?;

        // Rename is atomic on POSIX when source and destination are on the same filesystem
        if let Err(e) = std::fs::rename(&temp_ext_path, &ext_dir) {
            // Clean up temp file on failure
            let _ = std::fs::remove_file(&temp_ext_path);
            return Err(e).with_context(|| "Failed to rename extension to final destination");
        }

        // Smoke test: verify the extension loads correctly before enabling it in php.ini.
        // This prevents broken extensions from crashing PHP on startup.
        if !self.no_smoke_test {
            let smoke_test = Command::new("php")
                .arg("-d")
                .arg(format!("extension={}", ext_dir.display()))
                .arg("-r")
                .arg("")
                .output()
                .context("Failed to run PHP for smoke test")?;

            if !smoke_test.status.success() {
                // Extension failed to load - remove it and report the error
                let _ = std::fs::remove_file(&ext_dir);
                let stderr = String::from_utf8_lossy(&smoke_test.stderr);
                bail!(
                    "Extension failed to load during smoke test. The extension file has been removed.\n\
                     PHP output:\n{stderr}"
                );
            }
        }

        if let Some(php_ini) = php_ini {
            let file = File::open(&php_ini).with_context(|| "Failed to open `php.ini`")?;

            let ext_directive = format!("extension={ext_name}");

            // Check if a line is an extension directive for this extension (commented or not)
            let is_extension_directive = |line: &str| -> bool {
                line.trim()
                    .trim_start_matches([';', '#'])
                    .trim()
                    .strip_prefix("extension=")
                    .map(|s| s.trim_end_matches(consts::DLL_SUFFIX))
                    .is_some_and(|name| name == ext_name)
            };

            // Check if a line is an active (uncommented) extension directive
            let is_active_extension = |line: &str| -> bool {
                let trimmed = line.trim();
                if trimmed.starts_with(';') || trimmed.starts_with('#') {
                    return false;
                }
                trimmed
                    .strip_prefix("extension=")
                    .map(|s| s.trim_end_matches(consts::DLL_SUFFIX))
                    .is_some_and(|name| name == ext_name)
            };

            let mut new_lines = vec![];
            let mut already_enabled = false;
            for line in BufReader::new(&file).lines() {
                let line = line.with_context(|| "Failed to read line from `php.ini`")?;
                if is_active_extension(&line) {
                    already_enabled = true;
                }
                // Filter out any existing extension directives for this extension
                if !is_extension_directive(&line) {
                    new_lines.push(line);
                }
            }
            drop(file);

            // If already enabled and not trying to disable, nothing to do
            if already_enabled && !self.disable {
                println!("Extension already enabled in `php.ini`.");
                return Ok(());
            }

            // Add the extension line (commented if disable flag is set)
            let ext_line = if self.disable {
                format!(";{ext_directive}")
            } else {
                ext_directive
            };
            new_lines.push(ext_line);

            // Write atomically: write to temp file, then rename
            let tmp_path = php_ini.with_extension("ini.tmp");
            let mut tmp_file =
                File::create(&tmp_path).with_context(|| "Failed to create temporary file")?;
            tmp_file
                .write_all(new_lines.join("\n").as_bytes())
                .with_context(|| "Failed to write to temporary file")?;
            tmp_file.sync_all()?;
            drop(tmp_file);

            fs::rename(&tmp_path, &php_ini).with_context(|| "Failed to update `php.ini`")?;
        }

        Ok(())
    }
}

/// Copies an extension to the PHP extension directory.
///
/// # Parameters
///
/// * `ext_path` - Path to the built extension file.
/// * `install_dir` - Optional custom installation directory. If not provided,
///   the default PHP extension directory is used.
///
/// # Returns
///
/// The path where the extension was installed.
fn copy_extension(ext_path: &Utf8PathBuf, install_dir: Option<&PathBuf>) -> AResult<PathBuf> {
    let mut ext_dir = if let Some(dir) = install_dir {
        dir.clone()
    } else {
        get_ext_dir()?
    };

    debug_assert!(ext_path.is_file());
    let ext_name = ext_path.file_name().expect("ext path wasn't a filepath");

    if ext_dir.is_dir() {
        ext_dir.push(ext_name);
    }

    std::fs::copy(ext_path.as_std_path(), &ext_dir)
        .with_context(|| "Failed to copy extension from target directory to extension directory")?;

    Ok(ext_dir)
}

/// Returns the path to the extension directory utilised by the PHP interpreter,
/// creating it if one was returned but it does not exist.
fn get_ext_dir() -> AResult<PathBuf> {
    let cmd = Command::new("php")
        .arg("-r")
        .arg("echo ini_get('extension_dir');")
        .output()
        .context("Failed to call PHP")?;
    if !cmd.status.success() {
        bail!("Failed to call PHP: {cmd:?}");
    }
    let stdout = String::from_utf8_lossy(&cmd.stdout);
    let ext_dir = PathBuf::from(stdout.rsplit('\n').next().unwrap());
    if !ext_dir.is_dir() {
        if ext_dir.exists() {
            bail!(
                "Extension directory returned from PHP is not a valid directory: {}",
                ext_dir.display()
            );
        }

        std::fs::create_dir(&ext_dir).with_context(|| {
            format!(
                "Failed to create extension directory at {}",
                ext_dir.display()
            )
        })?;
    }
    Ok(ext_dir)
}

/// Returns the path to the `php.ini` loaded by the PHP interpreter.
fn get_php_ini() -> AResult<PathBuf> {
    let cmd = Command::new("php")
        .arg("-r")
        .arg("echo get_cfg_var('cfg_file_path');")
        .output()
        .context("Failed to call PHP")?;
    if !cmd.status.success() {
        bail!("Failed to call PHP: {cmd:?}");
    }
    let stdout = String::from_utf8_lossy(&cmd.stdout);
    let ini = PathBuf::from(stdout.rsplit('\n').next().unwrap());
    if !ini.is_file() {
        bail!(
            "php.ini does not exist or is not a file at the given path: {}",
            ini.display()
        );
    }
    Ok(ini)
}

impl Remove {
    pub fn handle(self) -> CrateResult {
        use std::env::consts;

        let artifact = find_ext(self.manifest.as_ref())?;

        let (mut ext_path, mut php_ini) = if let Some(install_dir) = self.install_dir {
            (install_dir, None)
        } else {
            (get_ext_dir()?, Some(get_php_ini()?))
        };

        if let Some(ini_path) = self.ini_path {
            php_ini = Some(ini_path);
        }

        let ext_file = format!(
            "{}{}{}",
            consts::DLL_PREFIX,
            artifact.name.replace('-', "_"),
            consts::DLL_SUFFIX
        );
        ext_path.push(&ext_file);

        if !ext_path.is_file() {
            bail!("Unable to find extension installed.");
        }

        if !self.yes
            && !Confirm::new()
                .with_prompt(format!(
                    "Are you sure you want to remove the extension `{}`?",
                    artifact.name
                ))
                .interact()?
        {
            bail!("Installation cancelled.");
        }

        std::fs::remove_file(ext_path).with_context(|| "Failed to remove extension")?;

        if let Some(php_ini) = php_ini.filter(|path| path.is_file()) {
            let file = File::open(&php_ini).with_context(|| "Failed to open `php.ini`")?;

            // The extension name without prefix/suffix (e.g., "my_ext" from "libmy_ext.so")
            let ext_name = artifact.name.replace('-', "_");

            // Check if a line is an extension directive for this extension
            // (matches both commented and uncommented lines like `extension=ext` or `; extension=ext`)
            let is_extension_directive = |line: &str| -> bool {
                line.trim()
                    .trim_start_matches([';', '#'])
                    .trim()
                    .strip_prefix("extension=")
                    .map(|s| s.trim_end_matches(consts::DLL_SUFFIX))
                    .is_some_and(|name| name == ext_name)
            };

            let mut new_lines = vec![];
            for line in BufReader::new(&file).lines() {
                let line = line.with_context(|| "Failed to read line from `php.ini`")?;
                if !is_extension_directive(&line) {
                    new_lines.push(line);
                }
            }
            drop(file);

            // Write atomically: write to temp file, then rename
            let tmp_path = php_ini.with_extension("ini.tmp");
            let mut tmp_file =
                File::create(&tmp_path).with_context(|| "Failed to create temporary file")?;
            tmp_file
                .write_all(new_lines.join("\n").as_bytes())
                .with_context(|| "Failed to write to temporary file")?;
            tmp_file.sync_all()?;
            drop(tmp_file);

            fs::rename(&tmp_path, &php_ini).with_context(|| "Failed to update `php.ini`")?;
        }

        Ok(())
    }
}

#[cfg(not(windows))]
impl Stubs {
    pub fn handle(self) -> CrateResult {
        use ext_php_rs::describe::ToStub;
        use std::{borrow::Cow, str::FromStr};

        let ext_path = if let Some(ext_path) = self.ext {
            ext_path
        } else {
            let target = find_ext(self.manifest.as_ref())?;
            build_ext(
                &target,
                false,
                self.features,
                self.all_features,
                self.no_default_features,
            )?
            .into()
        };

        if !ext_path.is_file() {
            bail!("Invalid extension path given, not a file.");
        }

        let ext = self::ext::Ext::load(ext_path)?;
        let result = ext.describe();

        // Ensure extension and CLI `ext-php-rs` versions are compatible.
        let cli_version = semver::VersionReq::from_str(ext_php_rs::VERSION).with_context(
            || "Failed to parse `ext-php-rs` version that `cargo php` was compiled with",
        )?;
        let ext_version = semver::Version::from_str(result.version).with_context(
            || "Failed to parse `ext-php-rs` version that your extension was compiled with",
        )?;

        if !cli_version.matches(&ext_version) {
            bail!(
                "Extension was compiled with an incompatible version of `ext-php-rs` - Extension: {ext_version}, CLI: {cli_version}"
            );
        }

        let stubs = result
            .module
            .to_stub()
            .with_context(|| "Failed to generate stubs.")?;

        if self.stdout {
            print!("{stubs}");
        } else {
            let out_path = if let Some(out_path) = &self.out {
                Cow::Borrowed(out_path)
            } else {
                let mut cwd = std::env::current_dir()
                    .with_context(|| "Failed to get current working directory")?;
                cwd.push(format!("{}.stubs.php", result.module.name));
                Cow::Owned(cwd)
            };

            std::fs::write(out_path.as_ref(), &stubs)
                .with_context(|| "Failed to write stubs to file")?;
        }

        Ok(())
    }
}

#[cfg(not(windows))]
impl Watch {
    #[allow(clippy::too_many_lines)]
    pub fn handle(self) -> CrateResult {
        use notify::RecursiveMode;
        use notify_debouncer_full::new_debouncer;
        use std::{
            process::Child,
            sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
                mpsc::channel,
            },
            time::Duration,
        };

        let artifact = find_ext(self.manifest.as_ref())?;
        let manifest_path = self.get_manifest_path()?;

        // Initial build and install
        println!("[cargo-php] Initial build...");
        let ext_path = build_ext(
            &artifact,
            self.release,
            self.features.clone(),
            self.all_features,
            self.no_default_features,
        )?;
        copy_extension(&ext_path, self.install_dir.as_ref())?;
        println!("[cargo-php] Build successful, extension installed.");

        // Start PHP server if requested
        let mut php_process: Option<Child> = if self.serve {
            Some(self.start_php_server()?)
        } else {
            None
        };

        // Start custom command if provided
        let has_command = !self.command.is_empty();
        let mut cmd_process: Option<Child> = if has_command {
            Some(self.start_command()?)
        } else {
            None
        };

        // Setup signal handler for graceful shutdown
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        })
        .context("Failed to set Ctrl+C handler")?;

        // Setup file watcher
        let (tx, rx) = channel();
        let mut debouncer = new_debouncer(Duration::from_millis(500), None, tx)
            .context("Failed to create file watcher")?;

        // Determine paths to watch
        let watch_paths = Self::determine_watch_paths(&manifest_path)?;
        for path in &watch_paths {
            debouncer
                .watch(path, RecursiveMode::Recursive)
                .with_context(|| format!("Failed to watch {}", path.display()))?;
        }

        println!("[cargo-php] Watching for changes... Press Ctrl+C to stop.");

        // Main watch loop
        while running.load(Ordering::SeqCst) {
            // Use a short timeout to periodically check the running flag
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(events)) => {
                    if !Self::is_relevant_event(&events) {
                        continue;
                    }

                    println!("\n[cargo-php] Change detected, rebuilding...");

                    // Kill PHP server if running
                    if let Some(mut process) = php_process.take() {
                        Self::kill_php_server(&mut process)?;
                    }

                    // Kill custom command if running
                    if let Some(mut process) = cmd_process.take() {
                        Self::kill_process(&mut process)?;
                    }

                    // Rebuild and install
                    match build_ext(
                        &artifact,
                        self.release,
                        self.features.clone(),
                        self.all_features,
                        self.no_default_features,
                    ) {
                        Ok(ext_path) => {
                            if let Err(e) = copy_extension(&ext_path, self.install_dir.as_ref()) {
                                eprintln!("[cargo-php] Failed to install extension: {e}");
                                eprintln!("[cargo-php] Waiting for changes...");
                            } else {
                                println!("[cargo-php] Build successful, extension installed.");

                                // Restart PHP server if in serve mode
                                if self.serve {
                                    match self.start_php_server() {
                                        Ok(process) => php_process = Some(process),
                                        Err(e) => {
                                            eprintln!(
                                                "[cargo-php] Failed to restart PHP server: {e}"
                                            );
                                        }
                                    }
                                }

                                // Restart custom command if provided
                                if has_command {
                                    match self.start_command() {
                                        Ok(process) => cmd_process = Some(process),
                                        Err(e) => {
                                            eprintln!("[cargo-php] Failed to restart command: {e}");
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[cargo-php] Build failed: {e}");
                            eprintln!("[cargo-php] Waiting for changes...");
                        }
                    }
                }
                Ok(Err(errors)) => {
                    for e in errors {
                        eprintln!("[cargo-php] Watch error: {e}");
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Just a timeout, continue checking running flag
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    bail!("File watcher channel disconnected");
                }
            }
        }

        // Cleanup on exit
        println!("\n[cargo-php] Shutting down...");
        if let Some(mut process) = php_process.take() {
            Self::kill_php_server(&mut process)?;
        }
        if let Some(mut process) = cmd_process.take() {
            Self::kill_process(&mut process)?;
        }

        Ok(())
    }

    fn get_manifest_path(&self) -> AResult<PathBuf> {
        if let Some(manifest) = &self.manifest {
            Ok(manifest.clone())
        } else {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            Ok(cwd.join("Cargo.toml"))
        }
    }

    fn determine_watch_paths(manifest_path: &std::path::Path) -> AResult<Vec<PathBuf>> {
        let project_root = manifest_path
            .parent()
            .context("Failed to get project root")?;

        let mut paths = vec![project_root.join("src"), manifest_path.to_path_buf()];

        // Add build.rs if it exists
        let build_rs = project_root.join("build.rs");
        if build_rs.exists() {
            paths.push(build_rs);
        }

        Ok(paths)
    }

    fn is_relevant_event(events: &[notify_debouncer_full::DebouncedEvent]) -> bool {
        events.iter().any(|event| {
            event.paths.iter().any(|path: &PathBuf| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "rs" || ext == "toml")
            })
        })
    }

    fn start_php_server(&self) -> AResult<std::process::Child> {
        let docroot = self
            .docroot
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new("."));

        let child = Command::new("php")
            .arg("-S")
            .arg(&self.host)
            .arg("-t")
            .arg(docroot)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to start PHP server")?;

        println!("[cargo-php] PHP server started on http://{}", self.host);
        Ok(child)
    }

    fn kill_php_server(process: &mut std::process::Child) -> AResult<()> {
        println!("[cargo-php] Stopping PHP server...");
        Self::kill_process(process)
    }

    fn start_command(&self) -> AResult<std::process::Child> {
        let cmd_str = self.command.join(" ");
        println!("[cargo-php] Running: {cmd_str}");

        let child = Command::new("sh")
            .arg("-c")
            .arg(&cmd_str)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("Failed to start command: {cmd_str}"))?;

        Ok(child)
    }

    fn kill_process(process: &mut std::process::Child) -> AResult<()> {
        use std::time::Duration;

        // Send SIGTERM on Unix
        #[allow(clippy::cast_possible_wrap)]
        unsafe {
            libc::kill(process.id() as i32, libc::SIGTERM);
        }

        // Wait up to 2 seconds for graceful shutdown
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(2);

        loop {
            if process.try_wait()?.is_some() {
                break;
            }
            if start.elapsed() > timeout {
                // Force kill after timeout
                process.kill()?;
                process.wait()?;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }
}

/// Attempts to find an extension in the target directory.
fn find_ext(manifest: Option<&PathBuf>) -> AResult<cargo_metadata::Target> {
    // TODO(david): Look for cargo manifest option or env
    let mut cmd = cargo_metadata::MetadataCommand::new();
    if let Some(manifest) = manifest {
        cmd.manifest_path(manifest);
    }

    let meta = cmd
        .features(cargo_metadata::CargoOpt::AllFeatures)
        .exec()
        .with_context(|| "Failed to call `cargo metadata`")?;

    let package = meta
        .root_package()
        .with_context(|| "Failed to retrieve metadata about crate")?;

    let targets: Vec<_> = package
        .targets
        .iter()
        .filter(|target| {
            target
                .crate_types
                .iter()
                .any(|ty| ty == &CrateType::DyLib || ty == &CrateType::CDyLib)
        })
        .collect();

    let target = match targets.len() {
        0 => bail!("No library targets were found."),
        1 => targets[0],
        _ => {
            let target_names: Vec<_> = targets.iter().map(|target| &target.name).collect();
            let chosen = Select::new()
                .with_prompt("There were multiple library targets detected in the project. Which would you like to use?")
                .items(&target_names)
                .interact()?;
            targets[chosen]
        }
    };

    Ok(target.clone())
}

/// Compiles the extension, searching for the given target artifact. If found,
/// the path to the extension dynamic library is returned.
///
/// # Parameters
///
/// * `target` - The target to compile.
/// * `release` - Whether to compile the target in release mode.
/// * `features` - Optional list of features.
///
/// # Returns
///
/// The path to the target artifact.
fn build_ext(
    target: &Target,
    release: bool,
    features: Option<Vec<String>>,
    all_features: bool,
    no_default_features: bool,
) -> AResult<Utf8PathBuf> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--message-format=json-render-diagnostics");
    if release {
        cmd.arg("--release");
    }
    if let Some(features) = features {
        cmd.arg("--features");
        for feature in features {
            cmd.arg(feature);
        }
    }

    if all_features {
        cmd.arg("--all-features");
    }

    if no_default_features {
        cmd.arg("--no-default-features");
    }

    let mut spawn = cmd
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| "Failed to spawn `cargo build`")?;
    let reader = BufReader::new(
        spawn
            .stdout
            .take()
            .with_context(|| "Failed to take `cargo build` stdout")?,
    );

    let mut artifact = None;
    for message in cargo_metadata::Message::parse_stream(reader) {
        let message = message.with_context(|| "Invalid message received from `cargo build`")?;
        match message {
            cargo_metadata::Message::CompilerArtifact(a) => {
                if &a.target == target {
                    artifact = Some(a);
                }
            }
            cargo_metadata::Message::BuildFinished(b) => {
                if b.success {
                    break;
                }

                bail!("Compilation failed, cancelling installation.")
            }
            _ => {}
        }
    }

    let artifact = artifact.with_context(|| "Extension artifact was not compiled")?;
    for file in artifact.filenames {
        if file.extension() == Some(std::env::consts::DLL_EXTENSION) {
            return Ok(file);
        }
    }

    bail!("Failed to retrieve extension path from artifact")
}
