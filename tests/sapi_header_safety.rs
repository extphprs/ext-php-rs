//! Verifies that raw SAPI header constructors are not exposed publicly.

use serde_json::Value;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const HEADER_CALL_LINE: u64 = 8;
const HEADERS_CALL_LINE: u64 = 9;

#[test]
fn raw_sapi_header_constructors_are_not_public() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = target_dir(&manifest_dir);
    let fixture = FixtureDir::new(&target_dir);
    let fixture_dir = fixture.path();
    let fixture_src_dir = fixture_dir.join("src");
    let fixture_target_dir = fixture_dir.join("target");
    fs::create_dir_all(&fixture_src_dir).expect("failed to create fixture source directory");

    let dependency_path = serde_json::to_string(
        manifest_dir
            .to_str()
            .expect("crate path must contain valid UTF-8"),
    )
    .expect("failed to quote dependency path");
    let fixture_manifest = format!(
        r#"[package]
name = "sapi-header-safety-fixture"
version = "0.0.0"
edition = "2024"

[workspace]

[dependencies]
ext-php-rs = {{ path = {dependency_path}, features = ["embed"] }}
"#,
    );
    fs::write(fixture_dir.join("Cargo.toml"), fixture_manifest)
        .expect("failed to write fixture manifest");

    let fixture_source = r"use ext_php_rs::embed::{SapiHeader, SapiHeaders};
use ext_php_rs::ffi::{sapi_header_struct, sapi_headers_struct};

fn main() {
    let header = std::ptr::null_mut::<sapi_header_struct>();
    let headers = std::ptr::null_mut::<sapi_headers_struct>();

    let _ = SapiHeader::from_raw(header);
    let _ = SapiHeaders::from_raw(headers);
}
";
    let fixture_main = fixture_src_dir.join("main.rs");
    fs::write(&fixture_main, fixture_source).expect("failed to write fixture source");
    let root_lock =
        fs::read(manifest_dir.join("Cargo.lock")).expect("failed to read root Cargo.lock");
    let root_lock_text =
        std::str::from_utf8(&root_lock).expect("root Cargo.lock is not valid UTF-8");
    assert!(
        !root_lock_text.contains("name = \"sapi-header-safety-fixture\""),
        "root Cargo.lock unexpectedly contains the generated fixture package"
    );
    let fixture_lock_path = fixture_dir.join("Cargo.lock");
    fs::write(&fixture_lock_path, &root_lock).expect("failed to copy root Cargo.lock into fixture");

    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let fixture_manifest_path = fixture_dir.join("Cargo.toml");
    let initial_output =
        run_fixture_check(&cargo, &fixture_manifest_path, &fixture_target_dir, false);
    let adapted_lock = fs::read(&fixture_lock_path)
        .expect("offline fixture check did not produce an adapted Cargo.lock");
    assert_ne!(
        adapted_lock, root_lock,
        "offline fixture check did not adapt the copied workspace lock"
    );
    audit_adapted_lock(root_lock_text, &adapted_lock);
    assert_expected_compile_failure(&initial_output, &fixture_main, "initial offline check");

    let locked_output =
        run_fixture_check(&cargo, &fixture_manifest_path, &fixture_target_dir, true);
    assert_expected_compile_failure(&locked_output, &fixture_main, "locked offline check");
    println!(
        "audited the offline-adapted lock and accepted exactly two E0599 diagnostics pinned to SapiHeader line {HEADER_CALL_LINE} and SapiHeaders line {HEADERS_CALL_LINE}"
    );
}

#[test]
fn fixture_directories_are_unique_and_cleaned_up() {
    let target_dir = target_dir(Path::new(env!("CARGO_MANIFEST_DIR")));
    let first_path;
    let second_path;
    {
        let first = FixtureDir::new(&target_dir);
        let second = FixtureDir::new(&target_dir);
        first_path = first.path().to_owned();
        second_path = second.path().to_owned();

        assert_ne!(first_path, second_path);
        assert!(first_path.is_dir());
        assert!(second_path.is_dir());
    }
    assert!(!first_path.exists());
    assert!(!second_path.exists());

    let mut panic_path = None;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let fixture = FixtureDir::new(&target_dir);
        panic_path = Some(fixture.path().to_owned());
        panic!("exercise fixture cleanup during unwinding");
    }));
    assert!(result.is_err());
    assert!(
        !panic_path
            .expect("panic fixture path was not captured")
            .exists()
    );
}

struct FixtureDir {
    path: PathBuf,
}

impl FixtureDir {
    fn new(parent: &Path) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);

        fs::create_dir_all(parent).expect("failed to create parent target directory");
        for _ in 0..100 {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                "sapi_header_safety_fixture-{}-{timestamp}-{id}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                Err(error) => panic!("failed to create unique fixture directory: {error}"),
            }
        }
        panic!("failed to create a unique fixture directory after 100 attempts");
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for FixtureDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn package_stanza<'a>(lock: &'a str, name: &str) -> &'a str {
    &lock[package_range(lock, name)]
}

fn package_range(lock: &str, name: &str) -> std::ops::Range<usize> {
    let marker = format!("[[package]]\nname = \"{name}\"\n");
    let start = unique_marker_index(lock, &marker, name);
    let after_marker = start + marker.len();
    let end = lock[after_marker..]
        .find("[[package]]\n")
        .map_or_else(|| lock.len(), |offset| after_marker + offset);
    start..end
}

fn unique_marker_index(lock: &str, marker: &str, package: &str) -> usize {
    let mut matches = lock.match_indices(marker);
    let index = matches.next().map_or_else(
        || panic!("root Cargo.lock has no package stanza for {package}"),
        |(index, _)| index,
    );
    assert!(
        matches.next().is_none(),
        "root Cargo.lock has duplicate package stanzas for {package}"
    );
    index
}

#[derive(Debug, Eq, PartialEq)]
struct RegistryPackageIdentity<'a> {
    name: &'a str,
    version: &'a str,
    source: &'a str,
    checksum: &'a str,
}

fn audit_adapted_lock(root_lock: &str, adapted_lock: &[u8]) {
    let adapted_lock = std::str::from_utf8(adapted_lock)
        .expect("offline-adapted fixture Cargo.lock is not valid UTF-8");
    for package in [
        "sapi-header-safety-fixture",
        "ext-php-rs",
        "ext-php-rs-build",
        "ext-php-rs-derive",
    ] {
        let _ = package_stanza(adapted_lock, package);
    }
    for removed_workspace_root in ["cargo-php", "tests"] {
        let marker = format!("[[package]]\nname = \"{removed_workspace_root}\"\n");
        assert!(
            !adapted_lock.contains(&marker),
            "offline-adapted lock retained unrelated workspace root {removed_workspace_root}"
        );
    }

    let root_registry = registry_package_identities(root_lock);
    for adapted in registry_package_identities(adapted_lock) {
        let matches = root_registry
            .iter()
            .filter(|root| *root == &adapted)
            .count();
        assert_eq!(
            matches, 1,
            "offline lock drift: adapted registry package has no unique exact root identity: {adapted:?}"
        );
    }
}

fn registry_package_identities(lock: &str) -> Vec<RegistryPackageIdentity<'_>> {
    lock.split("[[package]]\n")
        .skip(1)
        .filter_map(|stanza| {
            let source = lock_string_field(stanza, "source")?;
            source
                .starts_with("registry+")
                .then(|| RegistryPackageIdentity {
                    name: lock_string_field(stanza, "name")
                        .expect("registry package has no name in Cargo.lock"),
                    version: lock_string_field(stanza, "version")
                        .expect("registry package has no version in Cargo.lock"),
                    source,
                    checksum: lock_string_field(stanza, "checksum")
                        .expect("registry package has no checksum in Cargo.lock"),
                })
        })
        .collect()
}

fn lock_string_field<'a>(stanza: &'a str, field: &str) -> Option<&'a str> {
    let prefix = format!("{field} = \"");
    stanza
        .lines()
        .find_map(|line| line.strip_prefix(&prefix)?.strip_suffix('"'))
}

fn run_fixture_check(
    cargo: &OsString,
    manifest_path: &Path,
    target_dir: &Path,
    locked: bool,
) -> std::process::Output {
    let mut command = Command::new(cargo);
    command.arg("check");
    if locked {
        command.arg("--locked");
    }
    command
        .arg("--offline")
        .arg("--message-format=json")
        .arg("--manifest-path")
        .arg(manifest_path)
        .env("CARGO_TARGET_DIR", target_dir)
        .output()
        .expect("failed to run offline cargo check for fixture")
}

fn assert_expected_compile_failure(
    output: &std::process::Output,
    fixture_main: &Path,
    phase: &str,
) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let command_output = format!("{phase}\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !output.status.success(),
        "fixture unexpectedly compiled successfully\n{command_output}"
    );

    let errors: Vec<Value> = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-message")
        .filter_map(|message| message.get("message").cloned())
        .filter(|diagnostic| diagnostic["level"] == "error")
        .collect();

    assert_eq!(
        errors.len(),
        2,
        "expected exactly two compiler errors\n{command_output}"
    );
    assert!(
        errors.iter().all(|error| error["code"]["code"] == "E0599"),
        "expected only E0599 compiler errors\n{command_output}"
    );
    assert_diagnostic(
        &errors,
        fixture_main,
        HEADER_CALL_LINE,
        "SapiHeader",
        &command_output,
    );
    assert_diagnostic(
        &errors,
        fixture_main,
        HEADERS_CALL_LINE,
        "SapiHeaders",
        &command_output,
    );
}

fn target_dir(manifest_dir: &Path) -> PathBuf {
    match env::var_os("CARGO_TARGET_DIR").map(PathBuf::from) {
        Some(path) if path.is_absolute() => path,
        Some(path) => manifest_dir.join(path),
        None => manifest_dir.join("target"),
    }
}

fn assert_diagnostic(
    errors: &[Value],
    fixture_main: &Path,
    expected_line: u64,
    expected_type: &str,
    command_output: &str,
) {
    let matching: Vec<&Value> = errors
        .iter()
        .filter(|error| {
            let rendered = error["rendered"].as_str().unwrap_or_default();
            rendered.contains("from_raw")
                && rendered.contains(expected_type)
                && error["spans"].as_array().is_some_and(|spans| {
                    spans.iter().any(|span| {
                        span["is_primary"] == true
                            && span["line_start"].as_u64() == Some(expected_line)
                            && span["file_name"]
                                .as_str()
                                .is_some_and(|file| fixture_main.ends_with(file))
                    })
                })
        })
        .collect();

    assert_eq!(
        matching.len(),
        1,
        "expected one pinned E0599 diagnostic for {expected_type} on line {expected_line}\n{command_output}"
    );
}
