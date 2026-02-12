#![cfg_attr(windows, feature(abi_vectorcall))]
//! ext-php-rs-hotload: Hot-load Rust code into PHP
//!
//! This extension allows loading and compiling Rust code at runtime,
//! giving it full access to ext-php-rs capabilities. Compiled code is
//! cached on disk for fast subsequent loads.
//!
//! # Loading Methods
//!
//! ```php
//! // Load from file (global functions)
//! RustHotload::load('math.rs');
//! echo add(2, 3);
//!
//! // Load from string (global functions)
//! RustHotload::loadString('#[php_function] fn square(x: i64) -> i64 { x * x }');
//! echo square(5);
//!
//! // Single callable function
//! $triple = RustHotload::func('(x: i64) -> i64', 'x * 3');
//! echo $triple(14);  // 42
//!
//! // Rust class with state
//! $Counter = RustHotload::class('value: i64', '
//!     __construct(start: i64) { Self { value: start } }
//!     add(&mut self, n: i64) { self.value += n }
//!     get(&self) -> i64 { self.value }
//! ');
//! $c = $Counter(0);
//! $c->add(10);
//! echo $c->get();  // 10
//! ```
//!
//! # Build Mode
//!
//! ```php
//! RustHotload::setDebug(true);   // Debug builds (slower, debuggable)
//! RustHotload::setDebug(false);  // Release builds (fast, default)
//! ```
//!
//! # SAPI Compatibility
//!
//! Global functions (from `load()`/`loadString()`) are unregistered at request end,
//! but modules stay cached for fast re-registration. Scoped functions (from
//! `func()`/`class()`) are managed by their wrapper objects.
//!
//! Set `HOTLOAD_VERBOSE=1` to trace operations.

mod abi;
mod parser;

use abi::{PluginInfoFn, PluginInitFn};
use ext_php_rs::convert::IntoZval;
use ext_php_rs::ffi::zend_module_entry;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{ArrayKey, ZendCallable, Zval};
use ext_php_rs::zend::{ExecutorGlobals, ModuleEntry};
use ext_php_rs::{info_table_end, info_table_row, info_table_start};
use fs2::FileExt;
use libloading::{Library, Symbol};
use parking_lot::Mutex;
use parser::parse_plugin;
use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::fmt::Write as _;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use xxhash_rust::xxh3::xxh3_64;

/// Type for the ext-php-rs `get_module` function
type GetModuleFn = unsafe extern "C" fn() -> *mut ModuleEntry;

// Import the Zend API for module registration
extern "C" {
    fn zend_register_module_ex(module: *mut zend_module_entry) -> *mut zend_module_entry;
}

/// Functions registered as global in the current request (for cleanup)
static REQUEST_GLOBALS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn request_globals() -> &'static Mutex<HashSet<String>> {
    REQUEST_GLOBALS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// All loaded modules (persists across requests, never evicted)
static LOADED_MODULES: OnceLock<Mutex<HashMap<String, CachedModule>>> = OnceLock::new();

fn loaded_modules() -> &'static Mutex<HashMap<String, CachedModule>> {
    LOADED_MODULES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Modules currently being loaded (to prevent concurrent loading of same module)
static LOADING_MODULES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn loading_modules() -> &'static Mutex<HashSet<String>> {
    LOADING_MODULES.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Guard to ensure a module is removed from `loading_modules` on drop
struct LoadingGuard(String);

impl Drop for LoadingGuard {
    fn drop(&mut self) {
        loading_modules().lock().remove(&self.0);
    }
}

/// Debug mode flag (false = release, true = debug)
static DEBUG_MODE: OnceLock<Mutex<bool>> = OnceLock::new();

fn is_debug_mode() -> bool {
    *DEBUG_MODE.get_or_init(|| Mutex::new(false)).lock()
}

fn set_debug_mode(debug: bool) {
    *DEBUG_MODE.get_or_init(|| Mutex::new(false)).lock() = debug;
}

/// A cached module with its library handle and metadata
struct CachedModule {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    lib_path: PathBuf,
    /// Library handle - kept loaded to preserve registered functions
    #[allow(dead_code)]
    library: Library,
    functions: Vec<String>,
    /// Reference count - modules with refs can't be evicted
    ref_count: usize,
}

/// Find ext-php-rs path from environment or relative paths
fn find_ext_php_rs_path() -> PhpResult<PathBuf> {
    if let Ok(path) = std::env::var("EXT_PHP_RS_PATH") {
        return PathBuf::from(path)
            .canonicalize()
            .map_err(|_| PhpException::default("EXT_PHP_RS_PATH is invalid".to_string()));
    }

    if let Ok(cwd) = std::env::current_dir() {
        // Try to find it relative to current working directory
        if let Some(path) = [
            cwd.join("../.."),    // examples/hotload -> ext-php-rs
            cwd.join("../../.."), // deeper nesting
        ]
        .into_iter()
        .filter_map(|p| p.canonicalize().ok())
        .find(|p| p.join("Cargo.toml").exists())
        {
            return Ok(path);
        }
    }

    Err(PhpException::default(
        "Cannot find ext-php-rs. Set EXT_PHP_RS_PATH environment variable.".to_string(),
    ))
}

/// Get cache directory
fn cache_dir() -> PathBuf {
    std::env::var("PHP_RS_CACHE").map_or_else(
        |_| {
            let base = std::env::var("HOME").map_or_else(|_| PathBuf::from("/tmp"), PathBuf::from);
            base.join(".cache/ext-php-rs-hotload")
        },
        PathBuf::from,
    )
}

/// Get RUSTFLAGS for prefer-dynamic linking
///
/// Uses `-C prefer-dynamic` to link against system libstd.dylib instead of static linking.
/// This reduces module size from ~314KB to ~128KB at the cost of requiring Rust's libstd
/// to be available at runtime.
///
/// On macOS:
/// - Adds `-Wl,-undefined,dynamic_lookup` to allow PHP symbols to resolve at load time
/// - Adds `-Wl,-rpath,<path>` to embed the path to Rust's libstd.dylib
fn get_prefer_dynamic_rustflags() -> String {
    #[cfg(target_os = "macos")]
    {
        // Get Rust sysroot to find libstd.dylib
        let sysroot = Command::new("rustc")
            .arg("--print")
            .arg("sysroot")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        // Determine the target triple for the library path
        let target = if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        };

        let libstd_path = format!("{sysroot}/lib/rustlib/{target}/lib");

        format!(
            "-C prefer-dynamic -C link-args=-Wl,-undefined,dynamic_lookup -C link-args=-Wl,-rpath,{libstd_path}"
        )
    }

    #[cfg(target_os = "linux")]
    {
        // Get Rust sysroot to find libstd.so
        let sysroot = Command::new("rustc")
            .arg("--print")
            .arg("sysroot")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        // Determine the target triple for the library path
        let target = if cfg!(target_arch = "aarch64") {
            "aarch64-unknown-linux-gnu"
        } else {
            "x86_64-unknown-linux-gnu"
        };

        let libstd_path = format!("{sysroot}/lib/rustlib/{target}/lib");

        format!("-C prefer-dynamic -C link-args=-Wl,-rpath,{libstd_path}")
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "-C prefer-dynamic".to_string()
    }
}

/// Compute hash of byte content using xxh3 (fast non-cryptographic hash)
fn content_hash(content: &[u8]) -> String {
    format!("{:016x}", xxh3_64(content))
}

/// Resolve relative paths in a `path = "..."` dependency spec
///
/// If `source_dir` is provided and the path is relative, resolves it to an absolute path.
/// Returns the spec unchanged if the path is already absolute or no `source_dir` is provided.
fn resolve_path_dep(spec: &str, source_dir: Option<&Path>) -> String {
    // Extract the path from `path = "..."`
    let Some(start) = spec.find('"') else {
        return spec.to_string();
    };
    let Some(end) = spec[start + 1..].find('"') else {
        return spec.to_string();
    };
    let path_str = &spec[start + 1..start + 1 + end];
    let path = Path::new(path_str);

    // If path is absolute or no source_dir, return unchanged
    if path.is_absolute() || source_dir.is_none() {
        return spec.to_string();
    }

    // Resolve relative path against source_dir
    let source_dir = source_dir.unwrap();
    let resolved = source_dir.join(path);
    let resolved_str = resolved
        .canonicalize()
        .unwrap_or(resolved)
        .display()
        .to_string();

    // Rebuild the spec with the resolved path
    format!("path = \"{resolved_str}\"")
}

/// Generate handle-based object code from fields and methods
///
/// Returns (`rust_code`, `method_names`)
#[allow(clippy::too_many_lines)]
fn generate_object_code(
    fields: &str,
    methods: &str,
    prefix: &str,
    module_name: &str,
) -> (String, Vec<String>) {
    let mut method_names = Vec::new();
    let mut method_impls = String::new();
    let mut php_functions = String::new();

    // Parse methods
    let mut chars = methods.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        if chars.peek().is_none() {
            break;
        }

        // Read method name
        let mut method_name = String::new();
        while let Some(&c) = chars.peek() {
            if c == '(' {
                break;
            }
            method_name.push(c);
            chars.next();
        }
        let method_name = method_name.trim().to_string();

        if method_name.is_empty() {
            break;
        }

        // Read full signature until '{'
        let mut signature = String::new();
        let mut paren_depth = 0;
        while let Some(&c) = chars.peek() {
            if c == '{' && paren_depth == 0 {
                break;
            }
            if c == '(' {
                paren_depth += 1;
            } else if c == ')' {
                paren_depth -= 1;
            }
            signature.push(c);
            chars.next();
        }

        // Read body
        let mut body = String::new();
        let mut brace_depth = 0;
        for c in chars.by_ref() {
            if c == '{' {
                brace_depth += 1;
                if brace_depth > 1 {
                    body.push(c);
                }
            } else if c == '}' {
                brace_depth -= 1;
                if brace_depth == 0 {
                    break;
                }
                body.push(c);
            } else {
                body.push(c);
            }
        }

        method_names.push(method_name.clone());

        // Parse signature to extract parameters
        let sig_inner = signature.trim();
        let has_self = sig_inner.contains("&self") || sig_inner.contains("&mut self");
        let has_mut_self = sig_inner.contains("&mut self");
        let has_return = sig_inner.contains("->");

        // Extract return type
        let return_type = if has_return {
            sig_inner.split("->").nth(1).map_or("", str::trim)
        } else if method_name == "__construct" {
            "Self"
        } else {
            ""
        };

        // Extract params (remove &self/&mut self)
        let params_part = sig_inner.split("->").next().unwrap_or("");
        let params_inner = params_part
            .trim()
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or("");

        let params: Vec<&str> = params_inner
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "&self" && *s != "&mut self")
            .collect();

        if method_name == "__construct" {
            // Constructor: creates instance and stores in HashMap
            let param_list: String = params.join(", ");
            let _return_type_str = if return_type.is_empty() || return_type == "Self" {
                ""
            } else {
                return_type
            };

            let _ = write!(
                method_impls,
                "    pub fn __construct({}) -> Self {{ {} }}\n\n",
                param_list,
                body.trim()
            );

            // PHP function for constructor
            let php_param_list: String = params.join(", ");
            let arg_names: Vec<&str> = params
                .iter()
                .filter_map(|p| p.split(':').next())
                .map(str::trim)
                .collect();
            let arg_list = arg_names.join(", ");

            let _ = write!(
                php_functions,
                r#"
#[php_function]
#[php(name = "{prefix}___construct")]
pub fn ___construct({php_param_list}) -> i64 {{
    let instance = __Class::__construct({arg_list});
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    INSTANCES.lock().unwrap().insert(id, instance);
    id as i64
}}
"#
            );
        } else if has_self {
            // Instance method
            let param_list: String = params.join(", ");
            let self_param = if has_mut_self { "&mut self" } else { "&self" };
            let return_str = if return_type.is_empty() {
                String::new()
            } else {
                format!(" -> {return_type}")
            };

            let _ = write!(
                method_impls,
                "    pub fn {}({}{}{}){}{{ {} }}\n\n",
                method_name,
                self_param,
                if params.is_empty() { "" } else { ", " },
                param_list,
                return_str,
                body.trim()
            );

            // PHP function for method
            let php_param_list = if params.is_empty() {
                "id: i64".to_string()
            } else {
                format!("id: i64, {}", params.join(", "))
            };
            let arg_names: Vec<&str> = params
                .iter()
                .filter_map(|p| p.split(':').next())
                .map(str::trim)
                .collect();
            let arg_list = arg_names.join(", ");
            let return_str = if return_type.is_empty() {
                String::new()
            } else {
                format!(" -> {return_type}")
            };

            let get_method = if has_mut_self { "get_mut" } else { "get" };
            let call_args = if params.is_empty() {
                String::new()
            } else {
                arg_list
            };

            let _ = write!(
                php_functions,
                r#"
#[php_function]
#[php(name = "{prefix}_{method_name}")]
pub fn __{method_name}({php_param_list}){return_str} {{
    let mut guard = INSTANCES.lock().unwrap();
    if let Some(instance) = guard.{get_method}(&(id as u64)) {{
        instance.{method_name}({call_args})
    }} else {{
        panic!("Invalid object handle")
    }}
}}
"#
            );
        }
    }

    // Generate destroy function
    let _ = write!(
        php_functions,
        r#"
#[php_function]
#[php(name = "{prefix}___destroy")]
pub fn __destroy(id: i64) {{
    INSTANCES.lock().unwrap().remove(&(id as u64));
}}
"#
    );

    // Build function list for wrap_function
    let mut wrap_calls = vec![format!("        .function(wrap_function!(___construct))")];
    wrap_calls.push("        .function(wrap_function!(__destroy))".to_string());
    for name in &method_names {
        if name != "__construct" {
            wrap_calls.push(format!("        .function(wrap_function!(__{name}))"));
        }
    }

    let code = format!(
        r#"
use ext_php_rs::prelude::*;
use std::collections::HashMap;
use std::sync::{{Mutex, LazyLock}};
use std::sync::atomic::AtomicU64;

static INSTANCES: LazyLock<Mutex<HashMap<u64, __Class>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub struct __Class {{
    {fields}
}}

impl __Class {{
{method_impls}
}}

{php_functions}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {{
    module
{wrap_calls}
}}

// Hotload plugin ABI
#[repr(C)]
pub struct __HotloadInfo {{
    pub name: *const std::ffi::c_char,
    pub version: *const std::ffi::c_char,
    pub num_functions: u32,
    pub functions: *const (),
}}
unsafe impl Sync for __HotloadInfo {{}}

static __PLUGIN_INFO: __HotloadInfo = __HotloadInfo {{
    name: b"{module_name}\0".as_ptr() as *const std::ffi::c_char,
    version: b"1.0.0\0".as_ptr() as *const std::ffi::c_char,
    num_functions: 0,
    functions: std::ptr::null(),
}};

#[no_mangle]
pub extern "C" fn hotload_info() -> *const __HotloadInfo {{
    &__PLUGIN_INFO
}}

#[no_mangle]
pub extern "C" fn hotload_init() {{}}
"#,
        fields = fields,
        method_impls = method_impls,
        php_functions = php_functions,
        wrap_calls = wrap_calls.join("\n"),
        module_name = module_name,
    );

    (code, method_names)
}

/// Compile raw Rust source (already fully formed, no processing needed)
///
/// If `extra_source` is provided, it will be parsed for `use` statements
/// to extract dependencies and prepended to the source.
#[allow(clippy::too_many_lines)]
fn compile_raw_source(
    source: &str,
    extra_source: Option<&str>,
    module_name: &str,
    ext_php_rs_path: &Path,
    verbose: bool,
) -> Result<PathBuf, String> {
    // Parse extra_source for dependencies if provided
    let (deps, use_statements) = if let Some(extra) = extra_source {
        let parsed = parser::parse_plugin(extra, false);
        let uses: Vec<&str> = extra
            .lines()
            .filter(|l| l.trim().starts_with("use "))
            .collect();
        (parsed.dependencies, uses.join("\n"))
    } else {
        (HashMap::new(), String::new())
    };

    // Include use statements in hash for proper caching
    let full_source = if use_statements.is_empty() {
        source.to_string()
    } else {
        format!("{use_statements}\n\n{source}")
    };

    let hash = content_hash(full_source.as_bytes());
    let cache = cache_dir();
    let build_dir = cache.join(&hash);

    let package_name = format!("hotload_{hash}");
    #[cfg(target_os = "macos")]
    let lib_name = format!("lib{package_name}.dylib");
    #[cfg(target_os = "linux")]
    let lib_name = format!("lib{package_name}.so");
    #[cfg(target_os = "windows")]
    let lib_name = format!("{package_name}.dll");

    let debug = is_debug_mode();
    let profile = if debug { "debug" } else { "release" };
    // Use shared target directory for all modules to reuse compiled dependencies
    let shared_target = cache.join("target");
    let lib_path = shared_target.join(profile).join(&lib_name);

    // Fast path: already compiled
    if lib_path.exists() {
        if verbose {
            eprintln!("[hotload] Using cached: {}", lib_path.display());
        }
        return Ok(lib_path);
    }

    // Ensure cache directory exists for the lock file
    std::fs::create_dir_all(&cache).map_err(|e| format!("Failed to create cache dir: {e}"))?;

    // Acquire exclusive lock to prevent concurrent compilation of the same code
    let lock_path = cache.join(format!("{hash}.lock"));
    let lock_file =
        File::create(&lock_path).map_err(|e| format!("Failed to create lock file: {e}"))?;
    lock_file
        .lock_exclusive()
        .map_err(|e| format!("Failed to acquire lock: {e}"))?;

    // Double-check after acquiring lock (another process may have compiled it)
    if lib_path.exists() {
        if verbose {
            eprintln!("[hotload] Using cached: {}", lib_path.display());
        }
        return Ok(lib_path);
    }

    std::fs::create_dir_all(build_dir.join("src"))
        .map_err(|e| format!("Failed to create build dir: {e}"))?;

    // Build Cargo.toml with dependencies
    let mut cargo_toml = format!(
        r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2021"

[workspace]

[lib]
crate-type = ["cdylib"]

[dependencies]
ext-php-rs = {{ path = "{}" }}
"#,
        ext_php_rs_path.display()
    );

    // Add auto-detected dependencies from use statements
    for (crate_name, (version_or_spec, features)) in &deps {
        if version_or_spec.starts_with("path =") || version_or_spec.starts_with("git =") {
            // Full dependency spec (path or git)
            let _ = writeln!(cargo_toml, "{crate_name} = {{ {version_or_spec} }}");
        } else if let Some(feat) = features {
            let _ = writeln!(
                cargo_toml,
                "{crate_name} = {{ version = \"{version_or_spec}\", {feat} }}"
            );
        } else {
            let _ = writeln!(cargo_toml, "{crate_name} = \"{version_or_spec}\"");
        }
    }

    // Add release profile optimizations for smaller binaries
    // When HOTLOAD_PREFER_DYNAMIC is set, we use dynamic linking against libstd
    // otherwise we use full static optimization
    if std::env::var("HOTLOAD_PREFER_DYNAMIC").is_ok() {
        cargo_toml.push_str(
            r#"
[profile.release]
strip = true
codegen-units = 1
opt-level = "s"
"#,
        );
    } else {
        cargo_toml.push_str(
            r#"
[profile.release]
lto = true
strip = true
codegen-units = 1
opt-level = "s"
panic = "abort"
"#,
        );
    }

    std::fs::write(build_dir.join("Cargo.toml"), &cargo_toml)
        .map_err(|e| format!("Failed to write Cargo.toml: {e}"))?;

    std::fs::write(build_dir.join("src/lib.rs"), &full_source)
        .map_err(|e| format!("Failed to write lib.rs: {e}"))?;

    if verbose {
        let mode = if debug { "debug" } else { "release" };
        eprintln!("[hotload] Compiling {module_name} ({mode})...");
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if !debug {
        cmd.arg("--release");
    }

    // Optionally use prefer-dynamic to link against system libstd.dylib
    // This avoids duplicating ~200KB of std in each module
    if std::env::var("HOTLOAD_PREFER_DYNAMIC").is_ok() {
        let rustflags = get_prefer_dynamic_rustflags();
        cmd.env("RUSTFLAGS", &rustflags);
    }

    let output = cmd
        .arg("--manifest-path")
        .arg(build_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&shared_target)
        .output()
        .map_err(|e| format!("Failed to run cargo: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    if !lib_path.exists() {
        return Err("Compilation succeeded but library not found".to_string());
    }

    if verbose {
        eprintln!("[hotload] Built: {}", lib_path.display());
    }

    Ok(lib_path)
}

/// Process source code for hotloading:
/// 1. Adds `#[php_function]` to plain functions that don't have it
/// 2. Adds `#[php(name = "prefix_funcname")]` for name prefixing
fn apply_prefix(source: &str, prefix: &str) -> String {
    let mut result = String::with_capacity(source.len() + 512);
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Check for #[php_function] attribute
        if trimmed == "#[php_function]" || trimmed.starts_with("#[php_function(") {
            // Output the #[php_function] line
            result.push_str(line);
            result.push('\n');

            // Find the function name (skip over any additional attributes)
            let mut j = i + 1;
            while j < lines.len() {
                let next_trimmed = lines[j].trim();
                if next_trimmed.starts_with("#[") {
                    // Another attribute, skip it
                    j += 1;
                } else if let Some(fn_name) = extract_fn_name_from_line(lines[j]) {
                    // Found the function, add the name attribute
                    let indent = &line[..line.len() - line.trim_start().len()];
                    result.push_str(indent);
                    let _ = writeln!(result, "#[php(name = \"{prefix}_{fn_name}\")]");
                    break;
                } else {
                    break;
                }
            }
            i += 1;
            continue;
        }

        // Check for plain function without #[php_function]
        if let Some(fn_name) = extract_fn_name_from_line(line) {
            // Check if previous non-empty, non-attribute line was #[php_function]
            let has_php_function = has_php_function_attr(&lines, i);

            if !has_php_function {
                // Add #[php_function] and #[php(name = "...")] before the function
                let indent = &line[..line.len() - line.trim_start().len()];
                result.push_str(indent);
                result.push_str("#[php_function]\n");
                result.push_str(indent);
                let _ = writeln!(result, "#[php(name = \"{prefix}_{fn_name}\")]");
            }
        }

        result.push_str(line);
        result.push('\n');
        i += 1;
    }

    result
}

/// Check if a function at line index has `#[php_function]` attribute before it
fn has_php_function_attr(lines: &[&str], fn_line_idx: usize) -> bool {
    // Walk backwards from the function, skipping attributes
    let mut j = fn_line_idx.saturating_sub(1);
    loop {
        if j >= lines.len() {
            return false;
        }
        let trimmed = lines[j].trim();
        if trimmed.is_empty() {
            // Empty line - no more attributes
            return false;
        }
        if trimmed == "#[php_function]" || trimmed.starts_with("#[php_function(") {
            return true;
        }
        if trimmed.starts_with("#[") && trimmed.ends_with(']') {
            // Another attribute, keep looking
            if j == 0 {
                return false;
            }
            j -= 1;
        } else if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            // Doc comment, keep looking
            if j == 0 {
                return false;
            }
            j -= 1;
        } else {
            // Something else (code, etc.)
            return false;
        }
    }
}

/// Extract function name from a line like "fn foo(" or "pub fn foo("
fn extract_fn_name_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let after_fn = if trimmed.starts_with("pub fn ") {
        trimmed.strip_prefix("pub fn ")
    } else if trimmed.starts_with("fn ") {
        trimmed.strip_prefix("fn ")
    } else {
        None
    }?;

    after_fn.split('(').next().map(|s| s.trim().to_string())
}

/// Process source and generate Cargo.toml and final code using tree-sitter parser
#[allow(clippy::too_many_lines)]
fn process_plugin_source(
    source: &str,
    ext_php_rs_path: &Path,
    hash: &str,
    module_name: &str,
    source_dir: Option<&Path>,
) -> (String, String) {
    // Parse the source using tree-sitter
    // At this point, apply_prefix has already added #[php_function] to all functions that need it,
    // so we don't need auto_export here
    let parsed = parse_plugin(source, false);

    // Build Cargo.toml
    let package_name = format!("hotload_{hash}");
    let mut cargo_toml = format!(
        r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2021"

[workspace]

[lib]
crate-type = ["cdylib"]

[dependencies]
ext-php-rs = {{ path = "{}" }}
"#,
        ext_php_rs_path.display()
    );

    // Add auto-detected dependencies from use statements
    for (crate_name, (version_or_spec, features)) in &parsed.dependencies {
        if version_or_spec.starts_with("path =") {
            // Resolve relative paths against source directory
            let resolved_spec = resolve_path_dep(version_or_spec, source_dir);
            let _ = writeln!(cargo_toml, "{crate_name} = {{ {resolved_spec} }}");
        } else if version_or_spec.starts_with("git =") {
            // Git dependency (no path resolution needed)
            let _ = writeln!(cargo_toml, "{crate_name} = {{ {version_or_spec} }}");
        } else if let Some(feat) = features {
            let _ = writeln!(
                cargo_toml,
                "{crate_name} = {{ version = \"{version_or_spec}\", {feat} }}"
            );
        } else {
            let _ = writeln!(cargo_toml, "{crate_name} = \"{version_or_spec}\"");
        }
    }

    // Add release profile optimizations for smaller binaries
    // When HOTLOAD_PREFER_DYNAMIC is set, we use dynamic linking against libstd
    // otherwise we use full static optimization
    if std::env::var("HOTLOAD_PREFER_DYNAMIC").is_ok() {
        cargo_toml.push_str(
            r#"
[profile.release]
strip = true
codegen-units = 1
opt-level = "s"
"#,
        );
    } else {
        cargo_toml.push_str(
            r#"
[profile.release]
lto = true
strip = true
codegen-units = 1
opt-level = "s"
panic = "abort"
"#,
        );
    }

    // Build the final code
    let mut final_code = String::new();

    // Add doc comments first (they must be at the top of the crate)
    if !parsed.doc_comments.is_empty() {
        for line in &parsed.doc_comments {
            final_code.push_str(line);
            final_code.push('\n');
        }
    }

    // Add prelude if needed
    if !parsed.has_prelude {
        final_code.push_str("use ext_php_rs::prelude::*;\n\n");
    }

    // Add the original source (excluding doc comments we already added)
    let code_start = source
        .lines()
        .position(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with("//!")
        })
        .unwrap_or(0);
    for (i, line) in source.lines().enumerate() {
        if i >= code_start {
            final_code.push_str(line);
            final_code.push('\n');
        }
    }

    // Generate #[php_module] if needed
    if !parsed.has_php_module && !parsed.php_functions.is_empty() {
        let wrap_calls: Vec<String> = parsed
            .php_functions
            .iter()
            .map(|f| format!("        .function(wrap_function!({f}))"))
            .collect();

        let _ = write!(
            final_code,
            r"
// Auto-generated module registration
#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {{
    module
{}
}}
",
            wrap_calls.join("\n")
        );
    }

    // Add plugin boilerplate if needed
    if !parsed.has_plugin_boilerplate {
        // Generate function name strings
        let func_name_strs: Vec<String> = parsed
            .php_functions
            .iter()
            .enumerate()
            .map(|(i, name)| format!("static __FUNC_NAME_{i}: &[u8] = b\"{name}\\0\";"))
            .collect();

        // Generate function info array entries
        let func_info_entries: Vec<String> = (0..parsed.php_functions.len())
            .map(|i| {
                format!(
                    "    __FunctionInfo {{ name: __FUNC_NAME_{i}.as_ptr() as *const std::ffi::c_char }}"
                )
            })
            .collect();

        let num_functions = parsed.php_functions.len();
        let functions_ptr = if num_functions > 0 {
            "__PLUGIN_FUNCTIONS.as_ptr() as *const ()"
        } else {
            "std::ptr::null()"
        };

        let _ = write!(
            final_code,
            r#"
// Auto-generated plugin ABI
#[repr(C)]
pub struct __HotloadInfo {{
    pub name: *const std::ffi::c_char,
    pub version: *const std::ffi::c_char,
    pub num_functions: u32,
    pub functions: *const (),
}}
unsafe impl Sync for __HotloadInfo {{}}

#[repr(C)]
pub struct __FunctionInfo {{
    pub name: *const std::ffi::c_char,
}}
unsafe impl Sync for __FunctionInfo {{}}

{func_name_strs}

static __PLUGIN_FUNCTIONS: [__FunctionInfo; {num_functions}] = [
{func_info_entries}
];

static __PLUGIN_INFO: __HotloadInfo = __HotloadInfo {{
    name: b"{module_name}\0".as_ptr() as *const std::ffi::c_char,
    version: b"1.0.0\0".as_ptr() as *const std::ffi::c_char,
    num_functions: {num_functions},
    functions: {functions_ptr},
}};

#[no_mangle]
pub extern "C" fn hotload_info() -> *const __HotloadInfo {{
    &__PLUGIN_INFO
}}

#[no_mangle]
pub extern "C" fn hotload_init() {{}}
"#,
            func_name_strs = func_name_strs.join("\n"),
            func_info_entries = func_info_entries.join(",\n"),
            num_functions = num_functions,
            functions_ptr = functions_ptr,
            module_name = module_name,
        );
    }

    (cargo_toml, final_code)
}

/// Compile Rust source code to a dynamic library
fn compile_source(
    source_content: &str,
    module_name: &str,
    ext_php_rs_path: &Path,
    source_dir: Option<&Path>,
    verbose: bool,
) -> Result<PathBuf, String> {
    compile_source_with_prefix(
        source_content,
        module_name,
        None,
        ext_php_rs_path,
        source_dir,
        verbose,
    )
}

/// Compile Rust source code with optional function name prefix
///
/// When `prefix` is provided (for `eval()`), all functions are auto-exported as PHP functions.
/// When `prefix` is None (for `load()`), only `#[php_function]` marked functions are exported.
fn compile_source_with_prefix(
    source_content: &str,
    module_name: &str,
    prefix: Option<&str>,
    ext_php_rs_path: &Path,
    source_dir: Option<&Path>,
    verbose: bool,
) -> Result<PathBuf, String> {
    // Apply prefix if provided
    let processed_source = if let Some(pfx) = prefix {
        apply_prefix(source_content, pfx)
    } else {
        source_content.to_string()
    };

    let hash = content_hash(processed_source.as_bytes());
    let cache = cache_dir();
    let build_dir = cache.join(&hash);

    // Library name includes hash for uniqueness
    let package_name = format!("hotload_{hash}");
    #[cfg(target_os = "macos")]
    let lib_name = format!("lib{package_name}.dylib");
    #[cfg(target_os = "linux")]
    let lib_name = format!("lib{package_name}.so");
    #[cfg(target_os = "windows")]
    let lib_name = format!("{package_name}.dll");

    let debug = is_debug_mode();
    let profile = if debug { "debug" } else { "release" };
    // Use shared target directory for all modules to reuse compiled dependencies
    let shared_target = cache.join("target");
    let lib_path = shared_target.join(profile).join(&lib_name);

    // Fast path: already compiled
    if lib_path.exists() {
        if verbose {
            eprintln!("[hotload] Using cached: {}", lib_path.display());
        }
        return Ok(lib_path);
    }

    // Ensure cache directory exists for the lock file
    std::fs::create_dir_all(&cache).map_err(|e| format!("Failed to create cache dir: {e}"))?;

    // Acquire exclusive lock to prevent concurrent compilation of the same code
    let lock_path = cache.join(format!("{hash}.lock"));
    let lock_file =
        File::create(&lock_path).map_err(|e| format!("Failed to create lock file: {e}"))?;
    lock_file
        .lock_exclusive()
        .map_err(|e| format!("Failed to acquire lock: {e}"))?;

    // Double-check after acquiring lock (another process may have compiled it)
    if lib_path.exists() {
        if verbose {
            eprintln!("[hotload] Using cached: {}", lib_path.display());
        }
        return Ok(lib_path);
    }

    std::fs::create_dir_all(build_dir.join("src"))
        .map_err(|e| format!("Failed to create build dir: {e}"))?;

    let (cargo_toml, rust_code) = process_plugin_source(
        &processed_source,
        ext_php_rs_path,
        &hash,
        module_name,
        source_dir,
    );

    std::fs::write(build_dir.join("Cargo.toml"), &cargo_toml)
        .map_err(|e| format!("Failed to write Cargo.toml: {e}"))?;

    std::fs::write(build_dir.join("src/lib.rs"), &rust_code)
        .map_err(|e| format!("Failed to write lib.rs: {e}"))?;

    if verbose {
        let mode = if debug { "debug" } else { "release" };
        eprintln!("[hotload] Compiling {module_name} ({mode})...");
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if !debug {
        cmd.arg("--release");
    }

    // Optionally use prefer-dynamic to link against system libstd.dylib
    // This avoids duplicating ~200KB of std in each module
    if std::env::var("HOTLOAD_PREFER_DYNAMIC").is_ok() {
        let rustflags = get_prefer_dynamic_rustflags();
        cmd.env("RUSTFLAGS", &rustflags);
    }

    let output = cmd
        .arg("--manifest-path")
        .arg(build_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&shared_target)
        .output()
        .map_err(|e| format!("Failed to run cargo: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    if !lib_path.exists() {
        return Err("Compilation succeeded but library not found".to_string());
    }

    if verbose {
        eprintln!("[hotload] Built: {}", lib_path.display());
    }

    Ok(lib_path)
}

/// Compile a plugin from file
fn compile_plugin(source: &Path, verbose: bool) -> Result<PathBuf, String> {
    let source_content =
        std::fs::read_to_string(source).map_err(|e| format!("Failed to read source: {e}"))?;

    let source_dir = source
        .parent()
        .unwrap_or(Path::new("."))
        .canonicalize()
        .unwrap_or_else(|_| source.parent().unwrap_or(Path::new(".")).to_path_buf());

    // Derive module name from filename (e.g., "plugin_math.rs" -> "math", "hello.rs" -> "hello")
    let module_name = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module")
        .strip_prefix("plugin_")
        .unwrap_or(
            source
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module"),
        );

    // ext-php-rs is at ../../ relative to examples/hotload/
    let ext_php_rs_path = source_dir
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| source_dir.join("../.."));

    compile_source(
        &source_content,
        module_name,
        &ext_php_rs_path,
        Some(&source_dir),
        verbose,
    )
}

/// Compile a Cargo project directory to a dynamic library
///
/// The directory must contain a valid Cargo.toml with a cdylib target.
/// The project must export `hotload_info()` and `get_module()` functions.
#[allow(clippy::too_many_lines)]
fn compile_directory(dir_path: &Path, verbose: bool) -> Result<PathBuf, String> {
    let cargo_toml_path = dir_path.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Err(format!(
            "Directory does not contain Cargo.toml: {}",
            dir_path.display()
        ));
    }

    // Read Cargo.toml to find package name
    let cargo_toml_content = std::fs::read_to_string(&cargo_toml_path)
        .map_err(|e| format!("Failed to read Cargo.toml: {e}"))?;

    // Parse package name from Cargo.toml (simple parsing)
    let package_name = cargo_toml_content
        .lines()
        .find(|line| line.trim().starts_with("name"))
        .and_then(|line| {
            line.split('=')
                .nth(1)
                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        })
        .ok_or_else(|| "Cannot find package name in Cargo.toml".to_string())?;

    // Determine library filename based on platform
    let lib_name = package_name.replace('-', "_");
    #[cfg(target_os = "macos")]
    let lib_filename = format!("lib{lib_name}.dylib");
    #[cfg(target_os = "linux")]
    let lib_filename = format!("lib{lib_name}.so");
    #[cfg(target_os = "windows")]
    let lib_filename = format!("{lib_name}.dll");

    let debug = is_debug_mode();
    let target_dir = if debug {
        "target/debug"
    } else {
        "target/release"
    };
    let lib_path = dir_path.join(target_dir).join(&lib_filename);

    // Check if rebuild is needed by comparing modification times
    let needs_rebuild = if lib_path.exists() {
        // Check if any source file is newer than the library
        let lib_mtime = lib_path
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        // Check Cargo.toml and src/ directory
        let check_paths = [
            cargo_toml_path.clone(),
            dir_path.join("src/lib.rs"),
            dir_path.join("Cargo.lock"),
        ];

        check_paths.iter().any(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .map(|mtime| mtime > lib_mtime)
                .unwrap_or(false)
        })
    } else {
        true
    };

    if !needs_rebuild {
        if verbose {
            eprintln!("[hotload] Using existing build: {}", lib_path.display());
        }
        return Ok(lib_path);
    }

    // Acquire exclusive lock to prevent concurrent compilation
    let lock_path = dir_path.join(".hotload.lock");
    let lock_file =
        File::create(&lock_path).map_err(|e| format!("Failed to create lock file: {e}"))?;
    lock_file
        .lock_exclusive()
        .map_err(|e| format!("Failed to acquire lock: {e}"))?;

    // Double-check after acquiring lock (another process may have just finished building)
    if lib_path.exists() {
        let lib_mtime = lib_path
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let still_needs_rebuild = [
            cargo_toml_path.clone(),
            dir_path.join("src/lib.rs"),
            dir_path.join("Cargo.lock"),
        ]
        .iter()
        .any(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .map(|mtime| mtime > lib_mtime)
                .unwrap_or(false)
        });

        if !still_needs_rebuild {
            if verbose {
                eprintln!("[hotload] Using existing build: {}", lib_path.display());
            }
            return Ok(lib_path);
        }
    }

    if verbose {
        let mode = if debug { "debug" } else { "release" };
        eprintln!("[hotload] Building {} ({mode})...", dir_path.display());
    }

    // Build the project
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if !debug {
        cmd.arg("--release");
    }
    let output = cmd
        .arg("--manifest-path")
        .arg(&cargo_toml_path)
        .output()
        .map_err(|e| format!("Failed to run cargo: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    if !lib_path.exists() {
        return Err(format!(
            "Compilation succeeded but library not found at: {}",
            lib_path.display()
        ));
    }

    if verbose {
        eprintln!("[hotload] Built: {}", lib_path.display());
    }

    Ok(lib_path)
}

/// Load a compiled library and register it with PHP
///
/// Keeps all loaded modules in memory for fast access.
/// - `global`: If true, functions are tracked for cleanup at request end.
///   If false, functions persist until explicitly unloaded.
///
/// Returns the module name for tracking.
fn load_library(lib_path: PathBuf, global: bool, verbose: bool) -> PhpResult<String> {
    // Load library to get module name
    let lib = unsafe { Library::new(&lib_path) }
        .map_err(|e| PhpException::default(format!("Failed to load library: {e}")))?;

    // Get plugin info to determine module name
    let info_fn: Symbol<PluginInfoFn> = unsafe { lib.get(b"hotload_info\0") }
        .map_err(|e| PhpException::default(format!("Module missing hotload_info: {e}")))?;

    let info = unsafe { &*info_fn() };
    let module_name = unsafe { CStr::from_ptr(info.name) }
        .to_str()
        .map_err(|_| PhpException::default("Invalid module name".to_string()))?
        .to_string();

    // Try to handle from cache or wait if another thread is loading this module
    loop {
        // Check if module is cached
        {
            let modules = loaded_modules().lock();
            if let Some(cached) = modules.get(&module_name) {
                return handle_cached_module(cached, &module_name, global, verbose);
            }
        }

        // Check if another thread is currently loading this module
        {
            let mut loading = loading_modules().lock();
            if loading.contains(&module_name) {
                // Another thread is loading - drop lock and wait
                drop(loading);
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue; // Retry from cache check
            }

            // Mark this module as being loaded by us
            loading.insert(module_name.clone());
        }

        // We have the loading lock - break out to perform the actual load
        break;
    }

    // Guard to ensure we remove from loading_modules even on error
    let _guard = LoadingGuard(module_name.clone());

    // Double-check cache after acquiring loading lock (another thread may have just finished)
    {
        let modules = loaded_modules().lock();
        if let Some(cached) = modules.get(&module_name) {
            return handle_cached_module(cached, &module_name, global, verbose);
        }
    }

    // Not cached - need to fully register

    // Call init if present
    if let Ok(init_fn) = unsafe { lib.get::<PluginInitFn>(b"hotload_init\0") } {
        unsafe { init_fn() };
    }

    // Get the ext-php-rs get_module function and register the module
    let get_module_fn: Symbol<GetModuleFn> = unsafe { lib.get(b"get_module\0") }
        .map_err(|e| PhpException::default(format!("Module missing get_module: {e}")))?;

    let module_entry = unsafe { get_module_fn() };
    if module_entry.is_null() {
        return Err(PhpException::default(
            "Module get_module returned null".to_string(),
        ));
    }

    // Register the module with PHP
    let registered = unsafe { zend_register_module_ex(module_entry.cast()) };
    if registered.is_null() {
        return Err(PhpException::default(
            "Failed to register module".to_string(),
        ));
    }

    if verbose {
        eprintln!("[hotload] Module '{module_name}' registered");
    }

    // Collect function names from the plugin info
    let mut function_names = Vec::new();
    for i in 0..info.num_functions {
        let func = unsafe { &*info.functions.add(i as usize) };
        let name = unsafe { CStr::from_ptr(func.name) }
            .to_str()
            .map_err(|_| PhpException::default("Invalid function name".to_string()))?;
        function_names.push(name.to_string());

        if verbose {
            eprintln!("[hotload] Function available: {name}");
        }
    }

    // Track global functions for cleanup at request end
    if global {
        let mut globals = request_globals().lock();
        for func_name in &function_names {
            globals.insert(func_name.to_lowercase());
        }
    }

    // Store module (keeps library loaded)
    let cached = CachedModule {
        name: module_name.clone(),
        lib_path,
        library: lib,
        functions: function_names,
        ref_count: 0,
    };

    loaded_modules().lock().insert(module_name.clone(), cached);

    Ok(module_name)
}

/// Handle a module that's already in cache
fn handle_cached_module(
    cached: &CachedModule,
    module_name: &str,
    global: bool,
    verbose: bool,
) -> PhpResult<String> {
    if global {
        let mut globals = request_globals().lock();
        let first_func = cached.functions.first().map(|s| s.to_lowercase());

        if first_func.as_ref().is_none_or(|f| globals.contains(f)) {
            // Already registered this request
            if verbose {
                eprintln!("[hotload] Module '{module_name}' already loaded");
            }
            return Ok(module_name.to_string());
        }

        // Need to re-register functions for this request
        if verbose {
            eprintln!("[hotload] Module '{module_name}' re-registering functions");
        }

        let get_module_fn: Symbol<GetModuleFn> = unsafe { cached.library.get(b"get_module\0") }
            .map_err(|e| PhpException::default(format!("Module missing get_module: {e}")))?;

        let module_entry = unsafe { get_module_fn() };
        if !module_entry.is_null() {
            unsafe { zend_register_module_ex(module_entry.cast()) };
        }

        // Track functions for cleanup
        for func_name in &cached.functions {
            globals.insert(func_name.to_lowercase());
            if verbose {
                eprintln!("[hotload] Function available: {func_name}");
            }
        }

        return Ok(module_name.to_string());
    }

    // Non-global load, functions already registered
    if verbose {
        eprintln!("[hotload] Module '{module_name}' already loaded");
    }

    Ok(module_name.to_string())
}

/// A Rust object instance with state and methods (or a factory when handle == -1)
///
/// When used as an instance (handle >= 0):
/// - Use like a regular PHP object: `$obj->method(args...)`
/// - When destroyed, the Rust instance is freed
///
/// When used as a factory (handle == -1, returned by `RustHotload::class()`):
/// - Call to create instances: `$factory(args...)` returns a new `HotloadObject`
/// - Methods cannot be called directly on the factory
#[php_class]
pub struct HotloadObject {
    /// The module name (for cleanup)
    module_name: String,
    /// The instance handle (ID in the Rust `HashMap`)
    handle: i64,
    /// The function prefix for this class
    prefix: String,
    /// Maps method names to internal function names
    methods: HashMap<String, String>,
}

#[php_impl]
impl HotloadObject {
    /// Magic method to create instances when used as a factory (handle == -1)
    ///
    /// # Errors
    ///
    /// Returns an error if the constructor cannot be called or fails.
    pub fn __invoke(&self, args: &[&Zval]) -> PhpResult<HotloadObject> {
        if self.handle != -1 {
            return Err(PhpException::default(
                "This object is not a factory".to_string(),
            ));
        }

        // Call the ___construct function to create instance
        let ctor_fn = format!("{}___construct", self.prefix);
        let callable = ZendCallable::try_from_name(&ctor_fn)
            .map_err(|_| PhpException::default(format!("Cannot call constructor '{ctor_fn}'")))?;

        let args_ref: Vec<&dyn ext_php_rs::convert::IntoZvalDyn> = args
            .iter()
            .map(|v| *v as &dyn ext_php_rs::convert::IntoZvalDyn)
            .collect();

        let result = callable
            .try_call(args_ref)
            .map_err(|e| PhpException::default(format!("Constructor failed: {e:?}")))?;

        let handle = result.long().ok_or_else(|| {
            PhpException::default("Constructor did not return a handle".to_string())
        })?;

        // Increment module ref count (prevents eviction while in use)
        if let Some(module) = loaded_modules().lock().get_mut(&self.module_name) {
            module.ref_count += 1;
        }

        Ok(HotloadObject {
            module_name: self.module_name.clone(),
            handle,
            prefix: self.prefix.clone(),
            methods: self.methods.clone(),
        })
    }

    /// Magic method to call methods on this object
    ///
    /// # Errors
    ///
    /// Returns an error if the method is not found or the call fails.
    ///
    /// # Panics
    ///
    /// Panics if converting the handle to a Zval fails (should not happen).
    #[allow(clippy::needless_pass_by_value)] // PHP arrays convert to Vec, not slice
    pub fn __call(&self, name: &str, args: Vec<(ArrayKey<'_>, &Zval)>) -> PhpResult<Zval> {
        if self.handle == -1 {
            return Err(PhpException::default(
                "Cannot call methods on a factory. Create an instance first.".to_string(),
            ));
        }

        let internal_name = self
            .methods
            .get(name)
            .ok_or_else(|| PhpException::default(format!("Method '{name}' not found")))?;

        let callable = ZendCallable::try_from_name(internal_name)
            .map_err(|_| PhpException::default(format!("Cannot call '{internal_name}'")))?;

        // Prepend handle to args
        let handle_zval = self.handle.into_zval(false).unwrap();
        let mut all_args: Vec<&dyn ext_php_rs::convert::IntoZvalDyn> = vec![&handle_zval];
        all_args.extend(
            args.iter()
                .map(|(_, v)| *v as &dyn ext_php_rs::convert::IntoZvalDyn),
        );

        callable
            .try_call(all_args)
            .map_err(|e| PhpException::default(format!("Call failed: {e:?}")))
    }

    /// Destructor - frees the Rust instance
    ///
    /// # Panics
    ///
    /// Panics if converting the handle to a Zval fails (should not happen).
    pub fn __destruct(&self) {
        // Skip cleanup for factory objects (handle == -1)
        if self.handle == -1 {
            // Factory destructor - just decrement ref count
            if let Some(module) = loaded_modules().lock().get_mut(&self.module_name) {
                module.ref_count = module.ref_count.saturating_sub(1);
            }
            return;
        }

        // Call the __destroy function to free the Rust instance
        let destroy_fn = format!("{}___destroy", self.prefix);
        if let Ok(callable) = ZendCallable::try_from_name(&destroy_fn) {
            let handle_zval = self.handle.into_zval(false).unwrap();
            let _ = callable.try_call(vec![&handle_zval as &dyn ext_php_rs::convert::IntoZvalDyn]);
        }

        // Decrement module ref count
        if let Some(module) = loaded_modules().lock().get_mut(&self.module_name) {
            module.ref_count = module.ref_count.saturating_sub(1);
        }
    }
}

/// A single callable Rust function
///
/// Use as a callable: `$fn(args...)`
/// When destroyed, the underlying module is unloaded.
#[php_class]
pub struct HotloadFunc {
    /// The module name (used for unloading)
    module_name: String,
    /// The internal (prefixed) function name
    internal_name: String,
}

#[php_impl]
impl HotloadFunc {
    /// Magic method to make this object callable
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying function cannot be called.
    pub fn __invoke(&self, args: &[&Zval]) -> PhpResult<Zval> {
        let callable = ZendCallable::try_from_name(&self.internal_name)
            .map_err(|_| PhpException::default(format!("Cannot call '{}'", &self.internal_name)))?;

        let args_ref: Vec<&dyn ext_php_rs::convert::IntoZvalDyn> = args
            .iter()
            .map(|v| *v as &dyn ext_php_rs::convert::IntoZvalDyn)
            .collect();

        callable
            .try_call(args_ref)
            .map_err(|e| PhpException::default(format!("Call failed: {e:?}")))
    }

    /// Destructor - decrements reference count
    pub fn __destruct(&self) {
        if let Some(module) = loaded_modules().lock().get_mut(&self.module_name) {
            module.ref_count = module.ref_count.saturating_sub(1);
        }
    }
}

/// The `RustHotload` class for loading Rust code into PHP
#[php_class]
#[derive(Default)]
pub struct RustHotload;

#[php_impl]
impl RustHotload {
    /// Load Rust code from a file and register functions globally
    ///
    /// Compiles the code on first load (cached for subsequent loads).
    /// Functions marked with `#[php_function]` become globally available.
    /// All modules are cleaned up at request shutdown.
    ///
    /// ```php
    /// RustHotload::loadFile('math.rs');
    /// echo add(2, 3);  // Functions are now global
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not found, compilation fails, or the
    /// library cannot be loaded.
    #[php(name = "loadFile")]
    pub fn load_file(path: &str, verbose: Option<bool>) -> PhpResult<bool> {
        let verbose = verbose.unwrap_or(false);
        let source_path = PathBuf::from(path);

        if !source_path.exists() {
            return Err(format!("File not found: {path}").into());
        }

        let lib_path = compile_plugin(&source_path, verbose).map_err(PhpException::default)?;

        load_library(lib_path, true, verbose)?;
        Ok(true)
    }

    /// Load a Rust module from a Cargo project directory
    ///
    /// The directory must contain a valid Cargo.toml with a cdylib target.
    /// The project should use ext-php-rs and export functions via `#[php_module]`.
    ///
    /// Unlike `load()`, the project is built in-place (not in the cache directory).
    /// Rebuilds automatically when source files change.
    ///
    /// ```php
    /// // Load a full Cargo project
    /// RustHotload::loadDir('/path/to/my-extension');
    /// echo my_function();  // Functions from the project are now available
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the directory is invalid, compilation fails, or the
    /// library cannot be loaded.
    pub fn load_dir(path: &str, verbose: Option<bool>) -> PhpResult<bool> {
        let verbose = verbose.unwrap_or(false);
        let dir_path = PathBuf::from(path);

        if !dir_path.is_dir() {
            return Err(format!("Not a directory: {path}").into());
        }

        let lib_path = compile_directory(&dir_path, verbose).map_err(PhpException::default)?;

        load_library(lib_path, true, verbose)?;
        Ok(true)
    }

    /// Load Rust code from a string and register functions globally
    ///
    /// Like `load()` but takes code as a string instead of a file path.
    /// Functions marked with `#[php_function]` become globally available.
    /// Compiled code is cached based on content hash.
    ///
    /// ```php
    /// RustHotload::loadString('
    ///     #[php_function]
    ///     fn add(a: i64, b: i64) -> i64 { a + b }
    /// ');
    /// echo add(2, 3);  // 5
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails or the library cannot be loaded.
    #[php(name = "loadString")]
    pub fn load_string(code: &str, verbose: Option<bool>) -> PhpResult<bool> {
        let verbose = verbose.unwrap_or(false);
        let ext_php_rs_path = find_ext_php_rs_path()?;

        let hash = content_hash(code.as_bytes());
        let module_name = format!("hotload_{hash}");

        // No prefix - functions are registered with their original names
        // No source_dir since code comes from string, not a file
        let lib_path = compile_source(code, &module_name, &ext_php_rs_path, None, verbose)
            .map_err(PhpException::default)?;

        load_library(lib_path, true, verbose)?;
        Ok(true)
    }

    /// Create a Rust class with struct fields and methods
    ///
    /// Takes struct fields and methods separately, returns an anonymous class.
    /// Call the returned class to create instances: `$Counter(5)`
    ///
    /// ```php
    /// $Counter = RustHotload::class(
    ///     'value: i64',
    ///     '
    ///     __construct(start: i64) { Self { value: start } }
    ///     add(&mut self, b: i64) { self.value += b }
    ///     get(&self) -> i64 { self.value }
    ///     '
    /// );
    /// $c = $Counter(5);  // Create instance
    /// $c->add(10);
    /// echo $c->get();  // 15
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails or the library cannot be loaded.
    #[php(name = "class")]
    pub fn class_(fields: &str, methods: &str, verbose: Option<bool>) -> PhpResult<HotloadObject> {
        let verbose = verbose.unwrap_or(false);
        let ext_php_rs_path = find_ext_php_rs_path()?;

        // Fields are passed as "field: Type, ..." - braces added internally
        let fields_inner = fields.trim();

        // Generate unique prefix based on content hash
        let combined = format!("{fields}{methods}");
        let hash = content_hash(combined.as_bytes());
        let prefix = format!("_c{}", &hash[..8]);
        let module_name = format!("hotload_class_{hash}");

        // Parse methods to get names and generate handle-based code
        let (rust_code, method_names) =
            generate_object_code(fields_inner, methods, &prefix, &module_name);

        if verbose {
            eprintln!("[hotload] Generated class code:\n{rust_code}");
        }

        // Compile (pass methods to parse use statements for dependencies)
        let lib_path = compile_raw_source(
            &rust_code,
            Some(methods),
            &module_name,
            &ext_php_rs_path,
            verbose,
        )
        .map_err(PhpException::default)?;

        let actual_module_name = load_library(lib_path, false, verbose)?;

        // Build method map
        let mut methods_map = HashMap::new();
        for name in method_names {
            if name != "__construct" {
                methods_map.insert(name.clone(), format!("{prefix}_{name}"));
            }
        }

        // Return a "factory" HotloadObject with handle -1 (indicates factory mode)
        // When invoked, it creates new instances
        Ok(HotloadObject {
            module_name: actual_module_name,
            handle: -1, // Factory mode
            prefix,
            methods: methods_map,
        })
    }

    /// Create a single callable Rust function
    ///
    /// Takes a function signature and body separately, returns a callable object.
    ///
    /// ```php
    /// $triple = RustHotload::func('(x: i64) -> i64', 'x * 3');
    /// echo $triple(14);  // 42
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails or the library cannot be loaded.
    pub fn func(signature: &str, body: &str, verbose: Option<bool>) -> PhpResult<HotloadFunc> {
        let verbose = verbose.unwrap_or(false);
        let ext_php_rs_path = find_ext_php_rs_path()?;

        // Extract use statements from body and put them before the function
        let mut use_statements = Vec::new();
        let mut body_lines = Vec::new();
        for line in body.lines() {
            if line.trim().starts_with("use ") {
                use_statements.push(line);
            } else {
                body_lines.push(line);
            }
        }

        // Combine use statements and function definition
        let uses = use_statements.join("\n");
        let actual_body = body_lines.join("\n");
        let code = if uses.is_empty() {
            format!("fn __func{signature} {{ {actual_body} }}")
        } else {
            format!("{uses}\n\nfn __func{signature} {{ {actual_body} }}")
        };

        // Generate a unique prefix based on content hash
        let hash = content_hash(code.as_bytes());
        let prefix = format!("_f{}", &hash[..8]);
        let module_name = format!("hotload_func_{hash}");

        let lib_path = compile_source_with_prefix(
            &code,
            &module_name,
            Some(&prefix),
            &ext_php_rs_path,
            None, // No source_dir for inline code
            verbose,
        )
        .map_err(PhpException::default)?;

        let actual_module_name = load_library(lib_path, false, verbose)?;

        // The function name is "__func" which gets prefixed to "{prefix}___func"
        let internal_name = format!("{prefix}___func");

        Ok(HotloadFunc {
            module_name: actual_module_name,
            internal_name,
        })
    }

    /// List all currently loaded module names
    #[must_use]
    pub fn list() -> Vec<String> {
        loaded_modules().lock().keys().cloned().collect()
    }

    /// Get the function names exported by a loaded module
    ///
    /// # Errors
    ///
    /// Returns an error if the module is not found.
    pub fn info(name: &str) -> PhpResult<Vec<String>> {
        let modules = loaded_modules().lock();
        let module = modules
            .get(name)
            .ok_or_else(|| PhpException::default(format!("Module '{name}' not found")))?;

        Ok(module.functions.clone())
    }

    /// Manually unload a module by name
    ///
    /// Removes the module's functions from PHP and drops the library handle.
    ///
    /// # Errors
    ///
    /// Returns an error if the module is not found or the function table is
    /// inaccessible.
    pub fn unload(name: &str) -> PhpResult<bool> {
        // Get the function table to remove functions
        if let Some(function_table) = ExecutorGlobals::get().function_table_mut() {
            let mut modules = loaded_modules().lock();

            if let Some(module) = modules.remove(name) {
                // Remove all functions registered by this module
                for func_name in &module.functions {
                    let lowercase_name = func_name.to_lowercase();
                    function_table.remove(lowercase_name.as_str());
                }
                Ok(true)
            } else {
                Err(format!("Module '{name}' not found").into())
            }
        } else {
            Err("Cannot access function table".into())
        }
    }

    /// Get the path to the compilation cache directory
    ///
    /// Compiled .dylib files are stored here for fast subsequent loads.
    /// Default: `~/.cache/ext-php-rs-hotload`
    #[php(name = "cacheDir")]
    #[must_use]
    pub fn cache_dir() -> String {
        cache_dir().display().to_string()
    }

    /// Clear the compilation cache
    ///
    /// Removes all cached .dylib files. Next load will recompile from source.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be deleted.
    #[php(name = "clearCache")]
    pub fn clear_cache() -> PhpResult<bool> {
        let cache = cache_dir();
        if cache.exists() {
            std::fs::remove_dir_all(&cache)
                .map_err(|e| PhpException::default(format!("Failed to clear cache: {e}")))?;
        }
        Ok(true)
    }

    /// Set debug mode (false = release, true = debug)
    ///
    /// Release mode is the default. Debug mode enables debug symbols
    /// and disables optimizations for easier debugging.
    #[php(name = "setDebug")]
    pub fn set_debug(debug: bool) {
        set_debug_mode(debug);
    }

    /// Check if debug mode is enabled
    #[php(name = "isDebug")]
    #[must_use]
    pub fn is_debug() -> bool {
        is_debug_mode()
    }
}

/// Module info displayed in `phpinfo()`
pub extern "C" fn php_info(_module: *mut ext_php_rs::ffi::zend_module_entry) {
    info_table_start!();
    info_table_row!("PHP-RS Host", "enabled");
    info_table_row!("Version", "0.1.0");
    info_table_row!("Cache Directory", cache_dir().display().to_string());
    info_table_end!();
}

/// Request shutdown handler
///
/// Called automatically by PHP at the end of each request (RSHUTDOWN phase).
/// Unregisters global functions (from load/loadString) while keeping modules cached.
/// Functions from `func()`/`class()` are managed by their wrapper objects.
///
/// Set `HOTLOAD_VERBOSE=1` environment variable to trace operations.
#[allow(clippy::must_use_candidate)] // PHP callback, return value handled by PHP
pub extern "C" fn request_shutdown(_type: i32, _module_number: i32) -> i32 {
    let verbose = std::env::var("HOTLOAD_VERBOSE").is_ok();
    let mut globals = request_globals().lock();

    if !globals.is_empty() {
        if verbose {
            eprintln!(
                "[hotload] Request shutdown: unregistering {} global functions",
                globals.len()
            );
        }

        if let Some(function_table) = ExecutorGlobals::get().function_table_mut() {
            for func_name in globals.iter() {
                if function_table.remove(func_name.as_str()).is_some() && verbose {
                    eprintln!("[hotload] Unregistered: {func_name}");
                }
            }
        }

        globals.clear();
    }

    if verbose {
        let cache_size = loaded_modules().lock().len();
        eprintln!("[hotload] Request complete ({cache_size} modules cached)");
    }

    0
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .info_function(php_info)
        .request_shutdown_function(request_shutdown)
        .class::<RustHotload>()
        .class::<HotloadObject>()
        .class::<HotloadFunc>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_path_dep_relative() {
        // Use a path that works on both Unix and Windows
        let source_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Relative path should be resolved (joined with source_dir)
        let result = resolve_path_dep("path = \"../my-crate\"", Some(&source_dir));
        // The result should start with "path = \"" and contain the source_dir
        assert!(result.starts_with("path = \""));
        assert!(result.ends_with("my-crate\""));
        // Should be an absolute path now (or at least joined)
        assert!(!result.contains("path = \"../"));
    }

    #[test]
    fn test_resolve_path_dep_absolute() {
        let source_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Create an absolute path that works on both platforms
        #[cfg(unix)]
        let abs_path = "path = \"/opt/my-crate\"";
        #[cfg(windows)]
        let abs_path = "path = \"C:\\opt\\my-crate\"";

        // Absolute path should remain unchanged
        let result = resolve_path_dep(abs_path, Some(&source_dir));
        assert_eq!(result, abs_path);
    }

    #[test]
    fn test_resolve_path_dep_no_source_dir() {
        // Without source_dir, paths remain unchanged
        let result = resolve_path_dep("path = \"../my-crate\"", None);
        assert_eq!(result, "path = \"../my-crate\"");
    }

    #[test]
    fn test_resolve_path_dep_malformed() {
        let source_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Missing quotes
        let result = resolve_path_dep("path = ../my-crate", Some(&source_dir));
        assert_eq!(result, "path = ../my-crate");

        // No closing quote
        let result = resolve_path_dep("path = \"../my-crate", Some(&source_dir));
        assert_eq!(result, "path = \"../my-crate");
    }
}
