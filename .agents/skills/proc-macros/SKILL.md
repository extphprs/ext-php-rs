---
name: proc-macros
description: Use when changing or reviewing the ext-php-rs procedural macros (crates/macros: #[php_class], #[php_impl], #[php_function], #[php_module], #[php_enum], #[php_interface], #[php_const], #[php_extern], #[php_impl_interface], #[php(...)] options, ZvalConvert, wrap_function!, zend_fastcall!), their error messages, the expansion (.expanded.rs) and trybuild (.stderr) snapshots, the macro docs copied from guide/src/macros, or the generated stubs and cargo-php.
---

# Procedural macros

## Where things live

- `crates/macros/src/lib.rs`: the entry points, and the macro docs (a copy of
  `guide/src/macros/*.md` between `// BEGIN DOCS FROM <file>.md` / `// END DOCS FROM
  <file>.md`). Its `#[cfg(test)] mod tests` runs every macro over
  `tests/expand/*.rs` through `runtime-macros`, for coverage: a new macro must be added
  to the lists there.
- One file per macro: `class.rs`, `impl_.rs`, `function.rs` (argument parsing and the
  generated handler, shared by functions, methods and constructors), `module.rs`,
  `enum_.rs`, `interface.rs`, `impl_interface.rs`, `constant.rs`, `extern_.rs`,
  `zval.rs` (`ZvalConvert`), `fastcall.rs`. `parsing.rs` has the `#[php(...)]`
  option parsing (darling) and PHP name casing; `helpers.rs`, `syn_ext.rs` shared
  helpers.
- The generated code calls into `ext-php-rs` (`src/internal/`, `src/builders/`,
  `src/args.rs`, `src/class.rs`). A change usually touches both crates; the root
  crate pins `ext-php-rs-derive` with `=`, so they always ship together.
- Stubs: the extension describes itself through `src/describe` and
  `crates/introspection`; `cargo php stubs` (`crates/cli`) renders them.

## Rules for macro changes

- **Every path gets the change.** Functions, methods, static methods, constructors and
  interface methods share most of `function.rs` but not all of it (constructors return
  `ConstructorResult`, variadics keep the `ArgParser` path). Check each one, and
  `#[php_impl_interface]`, which builds methods separately.
- **Errors point at the user's code.** Use `syn::Error::new_spanned`/`bail!` with the
  offending tokens, never `panic!` or a span-less error. Each new error gets a trybuild
  case. Generated code whose trait bounds depend on the user's type is
  `quote_spanned!` on that type, so rustc reports an unmet bound on the parameter, not
  on the attribute.
- **Generated code is hygienic.** Refer to the runtime crate as
  `::ext_php_rs::…`, to std as `::std::…`/`::core::…`, and never rely on what the
  user imported. Generated items that users don't name are `#[doc(hidden)]`.
- **Module-entry code runs without the Zend heap.** What the macros evaluate while
  `get_module` builds the module (names, docs, default values for stubs) runs before
  PHP's heap exists in embed and static builds: `run_php_embed` calls `get_module()`
  before `Embed::run`. Use Rust or libc allocations only, never `emalloc`
  (`StubLiteral` renders defaults in pure Rust). The spawned `php` lanes never show it.
- **Run the lanes the NTS matrix skips** before pushing a macro change: the tests
  crate under embed (`cargo test -p tests --lib --features
  closure,embed,anyhow,observer,smartstring,indexmap -- --test-threads=1`) and the
  ASan lane (skill `zend-memory`).
- **PHP-visible output is API.** PHP names (casing rules in `parsing.rs`), arginfo
  types, defaults, flags and stubs are what PHP code sees; a change to them is a
  breaking change (skill `semver-breaking-changes`).
- **Generated code must pass the workspace clippy** (`-W clippy::pedantic -D
  warnings`) in user crates: emit `#[allow(...)]` on generated items only for lints
  that the generated shape can't avoid, with a reason.
- **Hot paths.** The generated handler runs on every PHP call. Don't allocate or format
  per call (names, default values) when it can be done once at registration;
  `benches/` measures this (see `benches/README.md`). To count instructions per call,
  build `benches/ext` with `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`, run
  `valgrind --tool=callgrind --trace-children=yes php -n -dextension=<lib>
  benches/<script>.php 100000`, subtract an empty script's count and divide by the
  call count.

## Snapshots

- **Expansion snapshots**: `crates/macros/tests/expand/<name>.rs` with its
  `<name>.expanded.rs`, checked by `tests/macrotest_expand.rs` (needs `cargo-expand`;
  runs only on nightly, never on Windows):
  `cargo +nightly test --test macrotest_expand`. In nix, the nightly shell is `.#asan`,
  whose sanitizer `RUSTFLAGS` break proc-macro loading: `nix develop .#asan -c env -u
  RUSTFLAGS -u CFLAGS -u LDFLAGS cargo test --test macrotest_expand`. A missing `.expanded.rs` is written
  on the first run; `MACROTEST=overwrite` rewrites existing ones. **Read the diff of
  every changed `.expanded.rs` before committing it**: it is the review of what the
  macro now generates. A refactor that should not change output must leave them
  untouched.
- **Compile-fail snapshots**: `tests/ui/<case>.rs` + `<case>.stderr`, run by
  `tests/ui.rs` with trybuild. The `.stderr` text depends on the rustc version, so CI
  pins a toolchain (`toolchain:` in the `compile-fail` job of
  `.github/workflows/build.yml`). Run and regenerate on exactly that toolchain:
  `cargo +<pinned> test --test ui -- --ignored`, and
  `TRYBUILD=overwrite cargo +<pinned> test --test ui -- --ignored` for new cases.
  Bumping the pin regenerates all `.stderr` files in the same PR.
- **Stub snapshots**: `crates/cli/tests/stubs_snapshot.rs` builds the `hello_world`
  example and compares `cargo php stubs` output with an insta snapshot
  (`crates/cli/tests/snapshots/`); `crates/cli/src/snapshots/` holds the
  static-linking glue snapshots. Review with `cargo insta review` or
  `INSTA_UPDATE=always`, and read the diff.
- Runtime behaviour still needs an integration test (`tests/src/integration/`): a
  snapshot shows what is generated, not that PHP behaves correctly.

## Docs

- Macro docs are written in `guide/src/macros/*.md`, then copied with
  `tools/update_lib_docs.sh` (needs the nightly toolchain for its rustfmt pass; the
  lefthook pre-commit job runs it when either side is staged). Never edit the copy in
  `lib.rs`.
- The script's final rustfmt pass follows `mod` declarations and rewraps comments in
  other macro files (`class.rs`, `impl_interface.rs`, `parsing.rs`, `zval.rs`).
  Restore them and commit only `lib.rs`. The `macro docs` workflow reruns the script
  and fails on any diff in `lib.rs`.
- A new `#[php(...)]` option is documented on the macro's guide page, with a Rust
  example and the PHP that shows its effect.
- The copy turns `rust,no_run` into `rust,no_run,ignore`, so guide examples are only
  compiled by skeptic (`guide_tests`), not as rustdoc tests of the macros crate.
  skeptic links what it finds in `target/`: E0464 after builds with other feature sets
  needs `cargo clean -p ext-php-rs`, E0514 after a toolchain bump needs a full
  `cargo clean`.
