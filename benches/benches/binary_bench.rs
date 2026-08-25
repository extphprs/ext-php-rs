use std::{
    path::PathBuf,
    process::{Command, ExitStatus},
    sync::{LazyLock, Once},
};

static BUILD: Once = Once::new();

static BENCH_ROOT: LazyLock<PathBuf> =
    LazyLock::new(|| std::env::current_dir().expect("Could not get cwd"));

static EXT_LIB: LazyLock<String> = LazyLock::new(|| {
    BENCH_ROOT
        .join("ext/target/release/libbenches.so")
        .display()
        .to_string()
});

fn bench_script(name: &str) -> String {
    BENCH_ROOT.join("benches").join(name).display().to_string()
}

fn setup() {
    if std::env::var_os("CODSPEED_ENV").is_some() {
        return;
    }

    BUILD.call_once(|| {
        let manifest = BENCH_ROOT.join("ext/Cargo.toml");

        let mut command = Command::new("cargo");
        command.arg("build");
        command.arg("--manifest-path").arg(&manifest);
        command.arg("--release");

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

fn run_php(script: &str, cnt: usize) -> ExitStatus {
    let status = Command::new("php")
        .arg(format!("-dextension={}", *EXT_LIB))
        .arg(bench_script(script))
        .arg(cnt.to_string())
        .status()
        .expect("failed to execute php");

    assert!(status.success(), "{script} exited with {status}");

    status
}

#[divan::bench(args = [1, 10, 100_000])]
fn function_calls(cnt: usize) -> ExitStatus {
    run_php("function_call.php", cnt)
}

#[divan::bench(args = [1, 10, 100_000])]
fn callback_calls(cnt: usize) -> ExitStatus {
    run_php("callback_call.php", cnt)
}

#[divan::bench(args = [1, 10, 100_000])]
fn method_calls(cnt: usize) -> ExitStatus {
    run_php("method_call.php", cnt)
}

#[divan::bench(args = [1, 10, 100_000])]
fn static_method_calls(cnt: usize) -> ExitStatus {
    run_php("static_method_call.php", cnt)
}

#[divan::bench(args = [1, 10, 100_000])]
fn property_reads(cnt: usize) -> ExitStatus {
    run_php("property_read.php", cnt)
}

#[divan::bench(args = [1, 10, 100_000])]
fn property_writes(cnt: usize) -> ExitStatus {
    run_php("property_write.php", cnt)
}

#[divan::bench(args = [1, 10, 100_000])]
fn property_dumps(cnt: usize) -> ExitStatus {
    run_php("property_dump.php", cnt)
}

#[divan::bench(args = [1, 10, 100_000])]
fn array_str_ref_keys(cnt: usize) -> ExitStatus {
    run_php("array_str_ref_keys.php", cnt)
}

#[divan::bench(args = [1, 10, 100_000])]
fn array_interned_keys(cnt: usize) -> ExitStatus {
    run_php("array_interned_keys.php", cnt)
}

fn main() {
    setup();
    divan::main();
}
