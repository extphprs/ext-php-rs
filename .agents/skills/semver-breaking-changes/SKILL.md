---
name: semver-breaking-changes
description: Use when an ext-php-rs PR changes public API or behaviour that existing extensions rely on (a removed or renamed item, a changed signature or trait, a macro that accepts less or generates different PHP-visible behaviour), when deciding whether a change is breaking, when writing a migration guide, or when release-plz / cargo-semver-checks reports a major bump. Covers the `!` commit, guide/src/migration-guides/, `#[non_exhaustive]`, and the workspace crates' versions.
---

# Breaking changes and the migration guide

Releases are made by release-plz from conventional commits on `main`: it opens a
"chore: release" PR with the version bumps and `CHANGELOG.md`, and it runs
cargo-semver-checks to infer the bump. While the crates are `0.x`, a breaking change
bumps the minor (`0.16.5` → `0.17.0`). Contributors never bump versions or edit
`CHANGELOG.md` themselves.

A breaking change needs two things in its PR: **`!` in the PR title** (it becomes
the squashed commit, and the `[**breaking**]` changelog entry) and **a migration guide
section**. Neither may be left out because a check stayed green: cargo-semver-checks
only sees Rust API signatures.

## 1. Is it breaking?

**Breaking** (the Rust side):
- removed, renamed or moved public items (types, functions, methods, traits, trait
  methods, modules, re-exports in `prelude`, feature flags);
- changed signatures: new required parameters, different parameter or return types,
  new trait bounds, a lifetime that ties things together that didn't before, `unsafe`
  added to a function;
- a new required method on a public trait users implement (`RegisteredClass`,
  `FromZval`, `IntoZval`, `ZBoxable`, `GcTrace`, …), or a new supertrait;
- a new variant or field in a public enum or struct that is **not**
  `#[non_exhaustive]` (users may match on it exhaustively or build it with a literal);
- a macro that rejects input it accepted before (`#[php_class]`, `#[php_impl]`,
  `#[php(...)]` options), or whose generated code needs something new from the user's
  types;
- a raised minimum: PHP version, Rust version (MSRV), a required feature.

**Breaking** (the PHP side, invisible to cargo-semver-checks):
- what PHP code sees from an existing extension changes: a PHP name of a function,
  method, class, property or constant (case conversion, namespace), a parameter that
  becomes required or by-reference, a different exception class or message for the
  same error, a type declaration in arginfo that rejects values that passed before,
  different stubs from `cargo php stubs`;
- an existing API whose meaning changes: a default that now behaves differently, a
  conversion (`FromZval`) that now rejects or coerces differently, a call that now
  throws where it returned before.

**Not breaking**: additions (a new function, a new method on a type, a new
`#[non_exhaustive]` enum variant, a new optional macro option), bug fixes that only
turn undefined behaviour or a crash into an error, internal (`pub(crate)`,
`#[doc(hidden)]`) changes. A fix that changes observable behaviour users may rely on is
still breaking: when in doubt, treat it as breaking and ask.

## 2. Check it

- cargo-semver-checks (install with `cargo install cargo-semver-checks --locked`):
  `cargo semver-checks --baseline-rev origin/main -p ext-php-rs`. It builds both
  revisions against the local PHP, so run it with the same PHP for both.
- PHP-visible changes: diff `cargo php stubs` output and the expansion snapshots
  (skill `proc-macros`) between `main` and the branch.

## 3. Keep the break small

- Prefer adding over changing: a new function next to the old one, the old one
  `#[deprecated(note = "use … instead")]`, removed in a later breaking release.
- New public enums and option structs get `#[non_exhaustive]` up front (as
  `Error` and `BorrowError` are), so later additions don't break.
- Keep `struct` literals that users write compiling: put new state somewhere private
  rather than adding a public field (precedent: #856 kept `ClassProperty`'s fields).

## 4. Write the migration guide

- The guide for the next breaking release is `guide/src/migration-guides/v<next>.md`
  (`v0.17.md` while `main` is on `0.16.x`). If it exists, add a section to it; if not,
  create it and link it at the top of the `Migration Guides` list in
  `guide/src/SUMMARY.md`.
- One `##`/`###` section per change, named after what users touch ("Class
  Parameters", "`self_` Parameters"). Say what changed, then show the code: the old
  usage and the new one, as short Rust (and PHP, when PHP-visible) blocks. Link the
  guide page with the full rules instead of repeating them.
- Mark examples `rust,ignore` unless they compile as a standalone file (skeptic
  compiles `rust,no_run` blocks as `guide_tests`).
- Update the rest of the guide and the rustdoc for the new behaviour in the same PR
  (the macro pages are copied into `crates/macros/src/lib.rs` with
  `tools/update_lib_docs.sh`).

## 5. Title and description

- Title: `type(scope)!: …` (skill `pr-description`). If a non-breaking PR gains a
  break during review, change the title and reword every commit on the branch so
  `convco check` still passes.
- In the description, one line at the end says what breaks and links the migration
  guide section. Tick the "migration guide" checklist box.

## The workspace crates

`ext-php-rs` depends on `ext-php-rs-derive` (the macros, pinned with `=`),
`ext-php-rs-introspection` (the ABI-stable extension description, shared with
`cargo-php`) and `ext-php-rs-build` (`crates/php-build`); `cargo-php` (`crates/cli`)
reads extensions through `ext-php-rs-introspection`. That format is meant to be
ABI-stable (see the crate's description): a change that an already-installed
`cargo-php` can't read is breaking for both crates. A break in one crate's public API
is a break for that crate, even if `ext-php-rs` re-exports nothing of it: the `!` and
the guide still apply. release-plz bumps the dependants together.
