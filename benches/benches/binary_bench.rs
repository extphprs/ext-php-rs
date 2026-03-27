use std::{
    path::PathBuf,
    process::Command,
    sync::{LazyLock, Once},
};

use gungraun::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, Callgrind,
    FlamegraphConfig,
};

static BUILD: Once = Once::new();

static BENCH_ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::current_dir().expect("Could not get cwd")
});

static EXT_LIB: LazyLock<String> = LazyLock::new(|| {
    BENCH_ROOT
        .join("ext/target/release/libbenches.so")
        .display()
        .to_string()
});

fn bench_script(name: &str) -> String {
    BENCH_ROOT
        .join("benches")
        .join(name)
        .display()
        .to_string()
}

const CACHE_SIM: [&str; 3] = [
    "--I1=32768,8,64",
    "--D1=32768,8,64",
    "--LL=67108864,16,64",
];

fn setup() {
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

#[binary_benchmark]
#[bench::single_function_call(args = ("function_call.php", 1))]
#[bench::multiple_function_calls(args = ("function_call.php", 10))]
#[bench::lots_of_function_calls(args = ("function_call.php", 100_000))]
fn function_calls(script: &str, cnt: usize) -> gungraun::Command {
    setup();

    gungraun::Command::new("php")
        .arg(format!("-dextension={}", *EXT_LIB))
        .arg(bench_script(script))
        .arg(cnt.to_string())
        .build()
}

#[binary_benchmark]
#[bench::single_callback_call(args = ("callback_call.php", 1))]
#[bench::multiple_callback_calls(args = ("callback_call.php", 10))]
#[bench::lots_of_callback_calls(args = ("callback_call.php", 100_000))]
fn callback_calls(script: &str, cnt: usize) -> gungraun::Command {
    setup();

    gungraun::Command::new("php")
        .arg(format!("-dextension={}", *EXT_LIB))
        .arg(bench_script(script))
        .arg(cnt.to_string())
        .build()
}

binary_benchmark_group!(
    name = function;
    config = BinaryBenchmarkConfig::default()
        .tool(Callgrind::with_args([
            CACHE_SIM[0], CACHE_SIM[1], CACHE_SIM[2],
            "--collect-atstart=no",
            "--toggle-collect=*_internal_bench_function*handler*",
        ]).flamegraph(FlamegraphConfig::default()));
    benchmarks = function_calls
);

binary_benchmark_group!(
    name = callback;
    config = BinaryBenchmarkConfig::default()
        .tool(Callgrind::with_args([
            CACHE_SIM[0], CACHE_SIM[1], CACHE_SIM[2],
            "--collect-atstart=no",
            "--toggle-collect=*_internal_bench_callback_function*handler*",
        ]).flamegraph(FlamegraphConfig::default()));
    benchmarks = callback_calls
);

#[binary_benchmark]
#[bench::single_method_call(args = ("method_call.php", 1))]
#[bench::multiple_method_calls(args = ("method_call.php", 10))]
#[bench::lots_of_method_calls(args = ("method_call.php", 100_000))]
fn method_calls(script: &str, cnt: usize) -> gungraun::Command {
    setup();

    gungraun::Command::new("php")
        .arg(format!("-dextension={}", *EXT_LIB))
        .arg(bench_script(script))
        .arg(cnt.to_string())
        .build()
}

#[binary_benchmark]
#[bench::single_static_call(args = ("static_method_call.php", 1))]
#[bench::multiple_static_calls(args = ("static_method_call.php", 10))]
#[bench::lots_of_static_calls(args = ("static_method_call.php", 100_000))]
fn static_method_calls(script: &str, cnt: usize) -> gungraun::Command {
    setup();

    gungraun::Command::new("php")
        .arg(format!("-dextension={}", *EXT_LIB))
        .arg(bench_script(script))
        .arg(cnt.to_string())
        .build()
}

binary_benchmark_group!(
    name = method;
    config = BinaryBenchmarkConfig::default()
        .tool(Callgrind::with_args([
            CACHE_SIM[0], CACHE_SIM[1], CACHE_SIM[2],
            "--collect-atstart=no",
            "--toggle-collect=*PhpClassImplCollector*BenchClass*handler*",
        ]).flamegraph(FlamegraphConfig::default()));
    benchmarks = method_calls
);

binary_benchmark_group!(
    name = static_method;
    config = BinaryBenchmarkConfig::default()
        .tool(Callgrind::with_args([
            CACHE_SIM[0], CACHE_SIM[1], CACHE_SIM[2],
            "--collect-atstart=no",
            "--toggle-collect=*PhpClassImplCollector*BenchClass*handler*",
        ]).flamegraph(FlamegraphConfig::default()));
    benchmarks = static_method_calls
);

main!(
    binary_benchmark_groups = function, callback, method, static_method
);
