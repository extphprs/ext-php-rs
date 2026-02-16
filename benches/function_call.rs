//! Benchmarks for PHP function call overhead in ext-php-rs.
//!
//! This benchmark suite measures the performance overhead of calling PHP
//! functions from Rust using various approaches:
//!
//! - Standard `#[php_function]` with type conversion
//! - Raw function access (direct `zend_execute_data` access)
//! - Different argument types (primitives, strings, arrays)

#![cfg_attr(windows, feature(abi_vectorcall))]
#![allow(
    missing_docs,
    deprecated,
    clippy::uninlined_format_args,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::semicolon_if_nothing_returned,
    clippy::explicit_iter_loop,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::implicit_hasher,
    clippy::doc_markdown
)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ext_php_rs::builders::SapiBuilder;
use ext_php_rs::embed::{Embed, ext_php_rs_sapi_startup};
use ext_php_rs::ffi::{
    php_module_startup, php_request_shutdown, php_request_startup, sapi_startup,
};
use ext_php_rs::prelude::*;
use ext_php_rs::zend::try_catch_first;
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Once;

static INIT: Once = Once::new();
static mut INITIALIZED: bool = false;

/// Initialize PHP SAPI for benchmarks
fn ensure_php_initialized() {
    INIT.call_once(|| {
        let builder = SapiBuilder::new("bench", "Benchmark");
        let sapi = builder.build().unwrap().into_raw();
        let module = get_module();

        unsafe {
            ext_php_rs_sapi_startup();
            sapi_startup(sapi);
            php_module_startup(sapi, module);
            INITIALIZED = true;
        }
    });
}

/// Start a PHP request context for benchmarks
fn with_php_request<R: Default, F: FnMut() -> R>(mut f: F) -> R {
    ensure_php_initialized();

    unsafe {
        php_request_startup();
    }

    let result = try_catch_first(AssertUnwindSafe(&mut f)).unwrap_or_default();

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
    }

    result
}

// ============================================================================
// Standard #[php_function] implementations
// ============================================================================

/// Simple function that returns a constant - baseline for function call
/// overhead
#[php_function]
pub fn bench_noop() -> i64 {
    42
}

/// Function taking a single i64 argument
#[php_function]
pub fn bench_single_int(n: i64) -> i64 {
    n + 1
}

/// Function taking two i64 arguments
#[php_function]
pub fn bench_two_ints(a: i64, b: i64) -> i64 {
    a + b
}

/// Function taking a String argument
#[php_function]
pub fn bench_string(s: String) -> i64 {
    s.len() as i64
}

/// Function taking a Vec argument
#[php_function]
pub fn bench_vec(v: Vec<i64>) -> i64 {
    v.iter().sum()
}

/// Function taking a HashMap argument
#[php_function]
pub fn bench_hashmap(m: HashMap<String, i64>) -> i64 {
    m.values().sum()
}

/// Function taking multiple mixed arguments
#[php_function]
pub fn bench_mixed(a: i64, s: String, b: i64) -> i64 {
    a + b + s.len() as i64
}

// ============================================================================
// Raw function implementations using #[php(raw)] - zero overhead
// ============================================================================

use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;

/// Raw function - direct access to ExecuteData and Zval
/// This bypasses all argument parsing and type conversion
#[php_function]
#[php(raw)]
pub fn bench_raw_noop(_ex: &mut ExecuteData, retval: &mut Zval) {
    retval.set_long(42);
}

/// Raw function taking a single int - manual argument extraction
#[php_function]
#[php(raw)]
pub fn bench_raw_single_int(ex: &mut ExecuteData, retval: &mut Zval) {
    let n = unsafe { ex.get_arg(0) }
        .and_then(|zv| zv.long())
        .unwrap_or(0);
    retval.set_long(n + 1);
}

/// Raw function that avoids all allocation - demonstrates zero-copy access
#[php_function]
#[php(raw)]
pub fn bench_raw_two_ints(ex: &mut ExecuteData, retval: &mut Zval) {
    unsafe {
        let a = ex.get_arg(0).and_then(|zv| zv.long()).unwrap_or(0);
        let b = ex.get_arg(1).and_then(|zv| zv.long()).unwrap_or(0);
        retval.set_long(a + b);
    }
}

// ============================================================================
// Module registration
// ============================================================================

#[php_module]
pub fn build_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        // Standard functions with type conversion
        .function(wrap_function!(bench_noop))
        .function(wrap_function!(bench_single_int))
        .function(wrap_function!(bench_two_ints))
        .function(wrap_function!(bench_string))
        .function(wrap_function!(bench_vec))
        .function(wrap_function!(bench_hashmap))
        .function(wrap_function!(bench_mixed))
        // Raw functions - zero overhead
        .function(wrap_function!(bench_raw_noop))
        .function(wrap_function!(bench_raw_single_int))
        .function(wrap_function!(bench_raw_two_ints))
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_function_call_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("function_call_overhead");

    // ---- Standard functions (with type conversion) ----

    // Benchmark: noop function (baseline)
    group.bench_function("noop_standard", |b| {
        b.iter(|| {
            with_php_request(|| {
                let result = Embed::eval("bench_noop();").unwrap();
                black_box(result.long().unwrap())
            })
        })
    });

    // Benchmark: single int argument
    group.bench_function("single_int_standard", |b| {
        b.iter(|| {
            with_php_request(|| {
                let result = Embed::eval("bench_single_int(42);").unwrap();
                black_box(result.long().unwrap())
            })
        })
    });

    // Benchmark: two int arguments
    group.bench_function("two_ints_standard", |b| {
        b.iter(|| {
            with_php_request(|| {
                let result = Embed::eval("bench_two_ints(21, 21);").unwrap();
                black_box(result.long().unwrap())
            })
        })
    });

    // ---- Raw functions (zero overhead) ----

    // Benchmark: raw noop function
    group.bench_function("noop_raw", |b| {
        b.iter(|| {
            with_php_request(|| {
                let result = Embed::eval("bench_raw_noop();").unwrap();
                black_box(result.long().unwrap())
            })
        })
    });

    // Benchmark: raw single int argument
    group.bench_function("single_int_raw", |b| {
        b.iter(|| {
            with_php_request(|| {
                let result = Embed::eval("bench_raw_single_int(42);").unwrap();
                black_box(result.long().unwrap())
            })
        })
    });

    // Benchmark: raw two int arguments
    group.bench_function("two_ints_raw", |b| {
        b.iter(|| {
            with_php_request(|| {
                let result = Embed::eval("bench_raw_two_ints(21, 21);").unwrap();
                black_box(result.long().unwrap())
            })
        })
    });

    group.finish();
}

fn bench_type_conversion_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_conversion");

    // String conversion
    group.bench_function("string_short", |b| {
        b.iter(|| {
            with_php_request(|| {
                let result = Embed::eval("bench_string('hello');").unwrap();
                black_box(result.long().unwrap())
            })
        })
    });

    group.bench_function("string_long", |b| {
        b.iter(|| {
            with_php_request(|| {
                let result = Embed::eval("bench_string(str_repeat('x', 1000));").unwrap();
                black_box(result.long().unwrap())
            })
        })
    });

    // Vec conversion with different sizes
    for size in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("vec", size), size, |b, &size| {
            b.iter(|| {
                with_php_request(|| {
                    let code = format!("bench_vec(range(1, {}));", size);
                    let result = Embed::eval(&code).unwrap();
                    black_box(result.long().unwrap())
                })
            })
        });
    }

    // HashMap conversion with different sizes
    for size in [1, 10, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("hashmap", size), size, |b, &size| {
            b.iter(|| {
                with_php_request(|| {
                    let code = format!(
                        "$arr = []; for ($i = 0; $i < {}; $i++) {{ $arr['key'.$i] = $i; }} bench_hashmap($arr);",
                        size
                    );
                    let result = Embed::eval(&code).unwrap();
                    black_box(result.long().unwrap_or(0))
                })
            })
        });
    }

    group.finish();
}

fn bench_mixed_arguments(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_arguments");

    group.bench_function("mixed_3args", |b| {
        b.iter(|| {
            with_php_request(|| {
                let result = Embed::eval("bench_mixed(10, 'hello', 20);").unwrap();
                black_box(result.long().unwrap())
            })
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_function_call_overhead,
    bench_type_conversion_overhead,
    bench_mixed_arguments,
);
criterion_main!(benches);
