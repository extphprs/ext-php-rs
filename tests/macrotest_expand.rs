//! The macro expansions in `crates/macros/tests/expand` compile against
//! ext-php-rs and match their snapshots. Windows is skipped: `zend_fastcall!`
//! expands to `extern "vectorcall"` there.
#![cfg(not(windows))]

#[rustversion::attr(nightly, test)]
#[allow(dead_code)]
fn expanded_macros_match_snapshots() {
    macrotest::expand("crates/macros/tests/expand/*.rs");
}
