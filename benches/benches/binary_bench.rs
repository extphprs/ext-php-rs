use std::{
    process::Command,
    sync::{LazyLock, Once},
};

use gungraun::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, Callgrind,
    FlamegraphConfig,
};

static BUILD: Once = Once::new();
static EXT_TARGET_DIR: LazyLock<String> = LazyLock::new(|| {
    let mut dir = std::env::current_dir().expect("Could not get cwd");
    dir.push("target");
    dir.push("release");
    dir.display().to_string()
});

fn setup() {
    BUILD.call_once(|| {
        let mut command = Command::new("cargo");
        command.arg("build");

        command.arg("--release");

        // Build features list dynamically based on compiled features
        // Note: Using vec_init_then_push pattern here is intentional due to conditional compilation
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

#[binary_benchmark]
#[bench::single_function_call(args = ("benches/function_call.php", 1))]
#[bench::multiple_function_calls(args = ("benches/function_call.php", 10))]
#[bench::lots_of_function_calls(args = ("benches/function_call.php", 100_000))]
fn function_calls(path: &str, cnt: usize) -> gungraun::Command {
    setup();

    gungraun::Command::new("php")
        .arg(format!("-dextension={}/libbenches.so", *EXT_TARGET_DIR))
        .arg(path)
        .arg(cnt.to_string())
        .build()
}

#[binary_benchmark]
#[bench::single_callback_call(args = ("benches/callback_call.php", 1))]
#[bench::multiple_callback_calls(args = ("benches/callback_call.php", 10))]
#[bench::lots_of_callback_calls(args = ("benches/callback_call.php", 100_000))]
fn callback_calls(path: &str, cnt: usize) -> gungraun::Command {
    setup();

    gungraun::Command::new("php")
        .arg(format!("-dextension={}/libbenches.so", *EXT_TARGET_DIR))
        .arg(path)
        .arg(cnt.to_string())
        .build()
}

binary_benchmark_group!(
    name = function;
    benchmarks = function_calls
);

binary_benchmark_group!(
    name = callback;
    benchmarks = callback_calls
);

main!(
    config = BinaryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--instr-atstart=no", "--I1=32768,8,64", "--D1=32768,8,64", "--LL=67108864,16,64"])
        .flamegraph(FlamegraphConfig::default()));
    binary_benchmark_groups = function, callback
);
