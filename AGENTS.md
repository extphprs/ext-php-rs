# ext-php-rs: how to work in this repository

## Fixing bugs and writing features

These rules come first, before every other rule here:

1. **Fix the root cause.** Find why the bug happens and fix that, not the symptom or
   the one input that showed it. If the same mistake exists elsewhere, fix it there too.
2. **Think through every way the change could cause bugs**, in edge cases and ordinary
   ones: other callers of the changed code, every PHP version from 8.1 to 8.6, ZTS and
   NTS, debug and release PHP, Linux, macOS and Windows (`vectorcall`), the cdylib and
   the `embed` paths, module restarts, feature combinations (`closure`, `anyhow`,
   `observer`, `enum`, …), weak and strict mode, PHP subclasses of a Rust class,
   exceptions and zend bailouts in the middle of the call. Check each one with a test.
3. **Add an explicit test case for each of those cases**, not only for the reported bug.

## Pull requests

- Push the branch to your fork, then `gh pr create --repo extphprs/ext-php-rs --base main`.
- PRs are squash-merged, and release-plz builds `CHANGELOG.md` and the version bump
  from the squashed commit. So the **PR title is a conventional commit**
  (`fix(array): …`, `feat(class)!: …`, `perf(macro): …`), with a scope from the
  existing history (`git log --format=%s origin/main | head -50`).
- CI also runs `convco check origin/main..HEAD`, so **every commit on the branch**
  must be a conventional commit too, fixups included.
- Never edit `CHANGELOG.md` or the crate versions in `Cargo.toml` by hand: release-plz
  owns them (its "chore: release" PR).
- Then poll `gh pr checks <n> --repo extphprs/ext-php-rs`. For `gh run list`,
  `--commit <sha>` is reliable; `--branch` sometimes returns stale runs.
- **Description:** follow skill `pr-description`. The repository's PR template
  (Description + Checklist) stays.
- **Breaking change:** follow skill `semver-breaking-changes` (`!` in the title, a
  migration guide in `guide/src/migration-guides/`).

## Agents and subagents

- Do fixes, reviews and PR work in the main session. Spawn a subagent only when asked
  to or for a genuinely independent, bounded task, and say so first.
- To review several PRs, use one subagent with an ordered target list (most urgent
  first, saying which to drop if time runs short), not one subagent per PR.
- Never use the Workflow tool unless explicitly asked for a workflow.
- Parallel agents building this workspace contend for CPU and for the `target/` lock:
  one `cargo` per checkout at a time, and tell subagents the testing rules above
  explicitly.

## No workarounds

Never hide an issue instead of fixing it:
- no `#[allow(...)]`/`#[expect(...)]` to silence clippy pedantic, rustdoc or
  `missing_docs` unless the lint is wrong for that code, and then with a comment that
  says why;
- no `#[ignore]`, `#[cfg]` or feature gate to switch off a failing test, and no deleting
  an assertion that fails;
- no entries in `.lsan-suppressions.txt` for a real leak, no `USE_ZEND_ALLOC`/ASan
  options that make a report go away;
- no overwriting an `.expanded.rs`, `.stderr` or insta `.snap` snapshot so it matches
  wrong output: read the diff first, and only accept output that is correct;
- no hand edits to `docsrs_bindings.rs`, and no new `extern "C"` block or hard-coded
  struct offset to reach a Zend symbol that `allowed_bindings.rs` should list;
- no disabling a mago rule, a typos word or a lefthook job, and no `--no-verify`;
- no changing fixtures or PHP test scripts to dodge behaviour.

Fix the root cause in the code, in the bindings, or upstream. If a proper fix is out of
reach, say so and ask. List any workaround explicitly in the report.

## Building

- PHP is found through `$PHP` (else `php` on `PATH`) and `$PHP_CONFIG`. bindgen needs
  libclang (`LIBCLANG_PATH`). `build.rs` emits the `php8x`, `php_zts` and `php_debug`
  cfgs from the PHP it finds (skill `php-bindings`).
- The PHP version is baked into the build. `build.rs` reruns when `PHP`, `PHP_CONFIG`
  or `PATH` change, but not when the binary behind the same path changes (`brew link
  php@8.3`, a rebuilt PHP): then run `cargo clean -p ext-php-rs`, or the old bindings
  stay.
- docs.rs builds without PHP from the checked-in `docsrs_bindings.rs`. CI checks this
  path with `DOCS_RS= cargo build` after `cargo clean`.
- Rust version: the crate uses edition 2024. Windows needs nightly
  (`abi_vectorcall`), see the `#![cfg_attr(windows, feature(abi_vectorcall))]` lines.

## Tests

- **Tests must assert the change.** Every test a PR adds or changes for the bug or
  feature itself must fail on the base branch and pass on the PR, because it checks
  the exact behaviour being fixed or added. Prove it by running that single test on
  both (see above). Edge-case tests that guard the change may pass on the base. Ways a
  test silently asserts nothing:
  - a `.php` script that no `#[test]` runs: each integration module needs its
    `pub mod` in `tests/src/integration/mod.rs`, its `build_module` call in
    `tests/src/lib.rs`, and a `#[test]` in its `mod.rs` that calls
    `run_php("<dir>/<file>.php")`;
  - a test behind `#[cfg(feature = "embed")]`, `#[cfg(php84)]` or another cfg that
    is off in the run you used as proof (check the test appears in the `running N
    tests` list);
  - `run_php` only checks the exit status. A warning, a deprecation or a leak report
    on stderr does not fail it: use `run_php_capturing_stderr` and assert on the
    output (see `tests/src/integration/panic/mod.rs`);
  - an `assert()` in PHP only fires with `zend.assertions=1` and
    `assert.exception=1`: `run_php` passes both, a manual `php` command must too;
  - `var_dump`/`echo` in a test script asserts nothing: compare with `===` in an
    `assert()`, or check the exception class and message in a `try`/`catch`;
  - a Rust-side function the script never calls (a typo in the PHP name only fails at
    runtime, so the script must reach the call).
- Integration tests run the extension in a real `php` process: one directory per area
  under `tests/src/integration/` with a `mod.rs` (the Rust items and tests) and one or
  more `.php` scripts. `_utils.php` has the shared helpers (`assert_type_error`,
  `assert_exception_thrown`, …). A few tests also have an `_embed` twin that runs the
  same script through `Embed` (`run_php_embed`).
- The harness loads the extension from `<repo>/target/{debug,release}`, derived from
  the current directory. Don't set `CARGO_TARGET_DIR` for integration tests: the build
  goes there and `php` loads a stale or missing library from `target/`.
- Class, enum and INI registration only run at MINIT, which never runs inside `Embed`'s
  request: test them through the cdylib (`run_php`), not only through embed.
- New macro behaviour gets an expansion test in `crates/macros/tests/expand/` and, for
  every new error, a trybuild case in `tests/ui/` (skill `proc-macros`).
- Guide code blocks are compiled by skeptic (`guide_tests`): `rust,no_run` blocks must
  compile, `rust,ignore` blocks are not checked.
- PHP test scripts are formatted and linted with mago (`mago.toml`). Some rules are off
  on purpose there; read the reason before you write around one.

## Code style

- Match the surrounding code: comment density, naming, idiom.
- Comments are concise and only explain what isn't obvious from the code:
  - don't restate the code in prose;
  - don't mention approaches that were tried, considered, replaced or removed, or
    anything that only makes sense in the context of an agent's iteration history;
  - prefer deleting an unnecessary comment over expanding or rewording it;
  - document an idea once, where it naturally belongs.
- Every `unsafe` block or `unsafe fn` says why it is sound (`// SAFETY:` / `# Safety`),
  in terms of the Zend invariant it relies on, with a php-src link pinned to a commit
  when the invariant isn't obvious.
- Public items need doc comments (`missing_docs` is on). Rustdoc must build with
  `-D warnings`.
- Documentation (`guide/`, rustdoc): more examples, less text. Show behaviour with a
  Rust declaration and the PHP that uses it, whose lines end in their result
  (`// int(3)`, `// TypeError`, `// no error`), and keep the prose around it short.
  Check each result on the branch before writing it.
- The macro docs in `crates/macros/src/lib.rs` are a copy of `guide/src/macros/*.md`
  between `// BEGIN DOCS FROM` / `// END DOCS FROM` markers. Edit the guide, then run
  `tools/update_lib_docs.sh` (needs the nightly toolchain for rustfmt). Never edit the
  copy.
- If you update functionality, update the guide in the same PR.

## Debugging pitfalls

- A crash in the extension shows up as a killed `php` (the harness prints `status:
  signal: 11 (SIGSEGV)`, or SIGABRT for a failed Zend assertion; a shell shows exit 139
  or 134) with little output. Rerun the script with `php -n
  -dextension=<lib>` so no host extension or `php.ini` gets involved, then under `lldb
  -- php …` (macOS) or `gdb --args php …` (Linux).
- Release PHP builds compile Zend assertions out. A bug that "only" corrupts memory on
  a release build often fails a Zend assertion on a debug build: reproduce on debug PHP
  (CI's `setup-php` uses `debug: true`; nix `.#debug`; skill `zend-memory`).
- Zend's allocator hides use-after-free and leaks from ASan and valgrind. Run with
  `USE_ZEND_ALLOC=0` (skill `zend-memory`).
- `Embed` and `php` behave differently: Embed shares one process across tests, so state
  leaks from one test into the next, and MINIT ran before the test started.

## Shell, worktrees and git

- Write commands that work in both zsh (macOS default) and bash (usual on Linux); check
  with `echo $0` if unsure:
  - Quote globs meant for the tool, not the shell (`grep --include='*.rs'`). In zsh an
    unmatched glob aborts the whole `&&` chain ("no matches found"); in bash it is
    passed on literally, so the command quietly acts on a path that doesn't exist.
    Either way the files you expect may not be there. `ls` repro files before
    concluding anything from a missing result.
  - Brace variables next to a colon: `"${b}:branch"`, `"${r}:src/types/zval.rs"`. In
    zsh `$b:l`, `$r:t`, … are history modifiers and mangle refspecs; braces are
    harmless in bash.
  - Word-splitting a variable of args: zsh needs `${=args}`, bash splits unquoted
    `$args`. Prefer an array (`args=(a b); cmd "${args[@]}"`), which behaves the same
    in both.
- The working directory resets between Bash calls. Start every command that edits,
  commits, builds or tests in a worktree with `cd <worktree> &&` (or use absolute
  paths), and check `git status` in the intended worktree before committing.
- All worktrees of this repository share one stash list. Never use `git stash` there,
  and never pop or drop entries you didn't create. To compare with and without a
  change, use `git diff > f.patch` + `git apply -R`, or a second worktree.
- Put scratch worktrees and scratch crates under `/tmp` (on macOS it is
  `/private/tmp`) or the session scratchpad. `/tmp` may be wiped on reboot, so don't
  keep anything there that must survive.
- Each worktree has its own `target/`, so its first build compiles everything again
  (bindgen included). Don't share `target/` between worktrees through
  `CARGO_TARGET_DIR` when you run integration tests (see Tests).

## Issues

Avoid opening GitHub issues that do not provide a linked fix: always open a linked PR with a fix for the issue, unless it's an issue related to a big architectural question or any other *major* change requiring further discussion before implementation, or any other non-directly-actionable issue.  
