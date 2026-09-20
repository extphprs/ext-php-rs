//! Compile-fail fixtures for the proc-macro diagnostics.
//!
//! The `.stderr` snapshots depend on the rustc version, so CI runs this test
//! on one pinned toolchain with `cargo test --test ui -- --ignored`.

#[test]
#[ignore = "run explicitly on the pinned toolchain: cargo test --test ui -- --ignored"]
fn macro_diagnostics() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
