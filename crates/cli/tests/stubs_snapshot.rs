#![cfg(not(windows))]
#![allow(missing_docs)]

use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use cargo_metadata::Message;

#[test]
fn hello_world_stubs() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find workspace root");

    // Use a separate target directory to avoid polluting the main target/
    // with artifacts built under different feature sets (which causes
    // "multiple candidates for rmeta dependency" errors in other tests).
    let target_dir = workspace_root.join("target").join("tests");

    // Build the hello_world example as cdylib
    let build_output = Command::new("cargo")
        .current_dir(workspace_root)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "build",
            "--example",
            "hello_world",
            "--message-format=json-render-diagnostics",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .expect("Failed to run cargo build");

    assert!(
        build_output.status.success(),
        "Failed to build hello_world example: {}",
        String::from_utf8_lossy(&build_output.stdout)
    );

    let reader = BufReader::new(build_output.stdout.as_slice());
    let lib_path = Message::parse_stream(reader)
        .filter_map(Result::ok)
        .find_map(|msg| match msg {
            Message::CompilerArtifact(artifact) if artifact.target.name == "hello_world" => {
                artifact
                    .filenames
                    .into_iter()
                    .find(|f| f.extension() == Some(std::env::consts::DLL_EXTENSION))
            }
            _ => None,
        })
        .expect("Failed to find hello_world cdylib artifact in cargo output");

    // Run cargo-php stubs --stdout against the built cdylib
    let stubs_output = Command::new("cargo")
        .current_dir(workspace_root)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args(["run", "-p", "cargo-php", "--", "stubs", "--stdout"])
        .arg(lib_path.as_str())
        .output()
        .expect("Failed to run cargo-php stubs");

    assert!(
        stubs_output.status.success(),
        "cargo-php stubs failed: {}",
        String::from_utf8_lossy(&stubs_output.stderr)
    );

    let stubs = String::from_utf8(stubs_output.stdout).expect("Invalid UTF-8 in stubs output");
    insta::assert_snapshot!(stubs);
}
