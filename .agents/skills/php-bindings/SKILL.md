---
name: php-bindings
description: Use when ext-php-rs needs a Zend symbol it doesn't bind yet, when editing allowed_bindings.rs, src/wrapper.c/.h or src/ffi.rs, when regenerating docsrs_bindings.rs (tools/update_bindings.sh), when code must differ between PHP versions, ZTS/NTS or debug builds (php81…php86, php_zts, php_debug cfgs), when adding support for a new PHP version or API number (crates/php-build ApiVersion), or for build.rs, Windows (windows_build.rs, vectorcall, devel packs) and static-linking build problems, or an `undefined symbol` from `cargo php stubs`.
---

# Bindings, PHP versions and the build script

## How the bindings are made

- `build.rs` finds PHP (`$PHP`/`php`, `$PHP_CONFIG`), reads its API number and
  ZTS/debug flags through `crates/php-build` (`PHPInfo`, `ApiVersion`), then runs
  bindgen on `src/wrapper.h` with an allowlist: only the names in
  `allowed_bindings.rs` are generated. Extension authors can add their own through
  `EXT_PHP_RS_ALLOWED_BINDINGS=name1,name2`.
- `unix_build.rs` / `windows_build.rs` supply the include paths, defines and link
  arguments per platform. On Windows, `windows_build.rs` downloads the PHP devel pack
  for the detected version, TS/NTS and architecture. This repository links with
  `rust-lld` through `.cargo/config.toml`; `windows_build.rs` only warns when an
  extension builds with the MSVC linker.
- `src/wrapper.c` builds into a static `wrapper` library and holds C helpers for what
  bindgen can't bind: Zend macros and `static inline` functions
  (`ext_php_rs_zend_string_init`, `ext_php_rs_zend_object_release`, the globals
  accessors, `ext_php_rs_zend_try_catch`, …). Their Rust declarations are written by
  hand in `src/ffi.rs` (bindgen would put them under `#[link(name = "php")]`).
- docs.rs has no PHP: with `DOCS_RS` set, `build.rs` uses the checked-in
  `docsrs_bindings.rs` and sets fixed cfgs (`php_debug`, `php81`…`php85`; no
  `php86`, no `php_zts`), so code behind other cfgs is not documented there.

## Adding a Zend symbol

1. A function, type, constant or global that `php.h` (or a header `wrapper.h`
   includes) declares: add its name to `allowed_bindings.rs`, in the list's order
   style. A header that isn't included yet goes into `src/wrapper.h`.
2. A macro or `static inline` function: add an `ext_php_rs_*` helper to
   `src/wrapper.c`, its prototype to `src/wrapper.h`, and the declaration to the
   `#[link(name = "wrapper")]` block in `src/ffi.rs`. Keep the helper a thin wrapper
   around the macro, so the Zend semantics stay in Zend.
3. Regenerate `docsrs_bindings.rs` with `tools/update_bindings.sh` (docker with
   buildx; it builds a debug PHP 8.5 in a container and runs bindgen there) and commit
   the result. The lefthook `bindings` job runs it when `allowed_bindings.rs` is
   staged. Never edit `docsrs_bindings.rs` by hand; the docs.rs build of a symbol that
   is missing there fails.
   The regeneration also picks up upstream changes to symbols already listed (new
   fields in `zend_executor_globals`, …). Commit the whole file: the `docsrs bindings`
   workflow reruns the script and fails on any diff.
4. Rust code that uses a symbol only some PHP versions have is gated with the version
   cfgs (below), in the bindings and at each use.
5. `cargo php stubs` loads the extension without PHP: `crates/cli/src/main.rs`
   defines a null stub for each name in `allowed_bindings.rs`, and real allocator
   stubs (`_emalloc`, `_femalloc`, …). A PHP patch release can make a Zend inline that
   `src/wrapper.c` uses reference a new symbol, and `stubs_snapshot` then fails with
   `undefined symbol: <name>`. Find that inline in the php-src headers and add a real
   stub next to `_emalloc`.

Never declare a Zend function in a private `extern "C"` block or hard-code a struct
layout to avoid the allowlist: the layout differs across versions, ZTS and platforms,
and bindgen is what keeps it right.

## Version and build cfgs

- `php80` … `php86`: stacked, so a PHP 8.4 build has `php80`…`php84`. Gate code that
  needs a newer API with `#[cfg(php84)]`, and the older path with
  `#[cfg(not(php84))]`. The minimum is enforced in `build.rs` (`min_api_version`):
  PHP 8.1 with the default `enum` feature, else 8.0 (with an EOL warning).
- `php_zts`: thread-safe PHP. Globals go through TSRM (`ext_php_rs_executor_globals()`
  and friends), never a direct `static`.
- `php_debug`: a `--enable-debug` PHP (`ZEND_DEBUG`); struct layouts and some
  assertions differ.
- In C (`wrapper.c/.h`) the same versions are `EXT_PHP_RS_PHP_8x` defines.
- New cfgs must be declared through `ext_php_rs_build::emit_check_cfg()`; crates
  that use them (like `tests/`) declare them in `[lints.rust] unexpected_cfgs`.
- `EXT_PHP_RS_STATIC_EXT=1` builds for static linking into the PHP binary (guide
  `advanced/static_linking.md`); CI only compile-checks it. `cargo test` can't work in
  that mode: `wrapper.c` then references `_tsrm_ls_cache`, which a prebuilt `libphp`
  doesn't export, so test executables fail to link. A functional check needs the
  two-pass php-src build of the guide.

## Supporting a new PHP version

Precedent: "feat(php): support PHP 8.6" (#833) and "accept every PHP 8.6 API version up
to 20261112" (#841). The usual set of changes:

- `crates/php-build/src/lib.rs`: the `ApiVersion` variant with its API number
  (`PHP_API_VERSION`/`ZEND_MODULE_API_NO` of the release, `YYYY_MM_DD`), `max()`,
  `versions()`, `cfg_name`, `define_name`, the `TryFrom<u32>` ranges and
  `MAX_API_VERSION` (pre-releases bump the API number: accept the whole range up to
  the RC's number);
- the Zend API differences in `src/` behind the new cfg (renamed fields, new
  parameters, removed functions), and `allowed_bindings.rs` + regenerated
  `docsrs_bindings.rs` for new symbols;
- `windows_build.rs` if the devel pack name or URL scheme changed;
- CI: the PHP version in the `build.yml` matrix (with the exclusions for releases that
  have no Windows devel pack yet), `.github/action/musl/Dockerfile` and the musl
  matrix, the nix `php8x` shells in `flake.nix` (while nixpkgs lags behind, override
  `version` and `phpSrc` with the php-src tag, as `php86Pkg` does);
- the docs.rs bindings and the `DOCS_RS` cfgs in `build.rs`, only at GA:
  `tools/update_bindings.sh` downloads php.net releases, not RCs;
- the guide pages that list version-specific behaviour. `README.md` only states the
  minimum ("PHP 8.1 or later"), which changes when the oldest version is dropped
  (with `min_api_version` in `build.rs`).

Run the version's build in a matching environment (nix `.#php86`, docker, or CI);
then let the full CI matrix run.

## Engine traps

- At MINIT, `EG(class_table)` is still null: `init_executor()` sets it at request
  startup. Look classes up in `CG(class_table)`, as `src/builders/class.rs` does. A
  second internal class with the same name silently replaces the first
  (`zend_hash_update_ptr`).
- Read the php-src code of a status getter before you trust it.
  `php_output_get_status()` masks out `PHP_OUTPUT_ACTIVATED` on PHP 8.1 to 8.5, so
  `ext_php_rs_output_activated()` in `src/wrapper.c` reads `OG(flags)` itself.

## Windows

- Windows PHP uses `vectorcall` for `ZEND_FASTCALL` functions, so Windows builds need
  nightly (`#![cfg_attr(windows, feature(abi_vectorcall))]` in every crate that
  defines extension functions; `zend_fastcall!` picks the ABI). `wrapper.h` undefines
  `__clang__` around `zend_portability.h` so bindgen sees the MSVC definition.
- Windows-only failures in CI are real. Don't gate code off on Windows to make one go
  away; find the ABI, layout or linker difference.
