---
name: zend-memory
description: Use when changing or reviewing code that owns, borrows or frees PHP memory in ext-php-rs (Zval, ZBox/ZBoxable, ZendStr, ZendHashTable, ZendObject, ZendClassObject, refcounts, shallow_clone, persistent vs request memory, IntoZval/FromZval), the Rust value of a #[php_class] (borrow cells, ClassRef/ClassRefMut, payloads), the cycle collector (GcTrace, get_gc), MINIT tables, or when debugging a segfault, use-after-free, double free or leak, including ASan, LSan, valgrind and debug-PHP runs.
---

# Zend memory, ownership and the cycle collector

## Where things live

- `src/types/zval.rs`: `Zval`. Dropping a `Zval` releases what it holds
  (`change_type` → null first, then the release, as `zend_assign_to_variable` does:
  the release can run a destructor or a GC run that walks this very slot).
  `shallow_clone` copies the value and bumps the refcount.
- `src/boxed.rs`: `ZBox<T>`, an owning pointer released through `ZBoxable::free`
  (`ext_php_rs_zend_string_release` for `ZendStr`, `zend_array_destroy` for
  `ZendHashTable` except immutable arrays, `ext_php_rs_zend_object_release`
  for objects; the `ext_php_rs_*` helpers wrap Zend macros in `src/wrapper.c`).
  `ZBox::into_raw` hands ownership to C; `ZBox::from_raw` takes it back.
- `src/types/string/`, `src/types/array/`, `src/types/object.rs`,
  `src/types/class_object.rs`: `ZendStr`, `ZendHashTable`, `ZendObject`,
  `ZendClassObject<T>` (a `zend_object` with the Rust value in front of it).
- `src/types/payload.rs`: the borrow cell around a class's Rust value, `ClassRef` /
  `ClassRefMut`, `BorrowError` (thrown to PHP as `Error`).
- `src/gc.rs`: `GcRoots`, `GcTrace`; `#[php_class]` reports each field whose type
  implements `GcTrace` from the class's `get_gc` handler.
- `src/types/separated.rs`, `src/types/php_ref.rs`: by-value mutation (COW
  separation) and by-reference parameters.
- `src/alloc.rs`: `emalloc`/`efree` wrappers.
- Guide: `guide/src/types/zval_ownership.md`, `guide/src/advanced/minit_memory.md`,
  `guide/src/macros/classes.md` ("Access to the Rust Value", "PHP Values in Rust
  Fields"), `guide/src/migration-guides/v0.17.md`.

## Ownership rules to check in every change

- **Know who owns each pointer.** For every `*mut zend_*` that crosses the FFI
  boundary, write down whether the engine borrows it, takes it (no refcount bump:
  `ZVAL_COPY_VALUE`, `zend_declare_class_constant`, `zend_enum_add_case`,
  `zend_hash_update` with a value), or copies it (`ZVAL_COPY`, `zend_string_copy`).
  When the engine takes a value, the Rust side must not drop it: `mem::forget`,
  `ManuallyDrop`, or `ZBox::into_raw`.
- **Refcounted vs not.** Interned strings, immutable arrays and scalars have no
  refcount to bump (`Z_REFCOUNTED`, `GC_FLAGS & GC_IMMUTABLE`). Code that increments
  `gc.refcount` must check the `RefCounted` type flag first, as `shallow_clone` does.
- **Persistent vs request memory.** Anything that outlives the request (MINIT tables,
  module globals, class constants of internal classes, persistent strings) must be
  allocated persistently (`pemalloc(…, 1)`, `zend_string_init(…, 1)`) and freed with
  the persistent deallocator. Request memory is freed at request end by ZendMM even if
  Rust still points at it. `ZendStr` release honours `IS_STR_PERSISTENT`.
- **Release can run PHP.** Dropping a `Zval`, a `ZBox<ZendObject>` or a hash table can
  call a destructor, which can call back into the extension and reach the same
  object. Never release while holding a `&mut` into memory the destructor can reach,
  or while a `ClassRefMut` of that object is alive. Move the value out first.
- **Hashtable iteration.** A callback during iteration can modify the table. Don't
  keep a `&Zval` into a bucket across a call to PHP; buckets move on resize.
- **Separation.** Writing to an array or string that the caller also holds needs
  separation first (`SEPARATE_ARRAY`, `Zval::array_mut`, also reached through
  `Separated`), otherwise the caller's
  value changes.
- **Bailouts.** A PHP fatal error longjmps over Rust frames, and their destructors
  never run (`.lsan-suppressions.txt` lists the tests that do this on purpose). Code
  that calls PHP and owns Rust values goes through `try_catch`/`BailoutGuard`
  (`src/zend/try_catch.rs`, `src/zend/bailout_guard.rs`, guide
  `advanced/bailout_guard.md`).
- **MINIT tables** are owned for the life of the process by a per-extension static;
  method tables are freed right after registration (guide `advanced/minit_memory.md`).
  A `&'static ClassEntry` is valid only until module shutdown: a SAPI that restarts
  the module (FrankenPHP) frees and recreates every class entry, so never cache one
  across MINIT (same guide, "Module restarts").

## The Rust value of a class

- The Rust value of a `#[php_class]` object lives in a borrow cell. Generated
  wrappers borrow it for the whole call: `&self` takes a shared borrow, `&mut self` an
  exclusive one. A PHP call from a method that reaches the same object again gets a
  `BorrowError` (PHP `Error`), not a second `&mut`.
- An object of a PHP subclass that never calls `parent::__construct()` has no Rust
  value: `BorrowError::Uninitialized`. Every path that reads the value (methods,
  property handlers, `get_gc`, `clone`, `var_dump`, comparison) must handle it.
- Property handlers, `get_gc`, `clone_obj` and `free_obj` run from the engine at any
  time, including while a method holds the value mutably. They must not panic or
  double-borrow; the collector does not read a value that is mutably borrowed.

## The cycle collector

- A cycle is collected only if every object in it reports the values it holds.
  A field that keeps a PHP value must be of a `GcTrace` type (see the list in the
  guide), or the cycle leaks.
- A new container or wrapper type that can hold PHP values needs a `GcTrace` impl, and
  a test that builds a cycle through it, `unset`s it, calls `gc_collect_cycles()` and
  checks the count it returns or a destructor that ran.
- `get_gc` must report each value once, and must not allocate per call beyond the
  engine's GC buffer.

## Tests for memory bugs

- A test for a use-after-free or double free must fail on the base under a detector
  (ASan, debug PHP, valgrind), not only "by luck". Say which one in the test's
  comment if it doesn't crash without it.
- Refcounts: `debug_zval_dump()` in the PHP script shows them; assert on the output
  through `run_php_capturing_stderr` or by comparing `ob_get_clean()` in PHP.
- Leaks: a debug PHP reports request leaks on stderr at shutdown
  (`[Fri …] Script: '…'` / `=== Total N memory leaks detected ===`); `run_php` does
  not fail on them, so assert stderr is clean.

## Detectors

- **Debug PHP** (`--enable-debug`): Zend assertions and the leak report. CI's
  `setup-php` uses `debug: true`. Linux: `nix develop .#debug` / `.#zts-debug` /
  `.#php86-debug`.
- **`USE_ZEND_ALLOC=0`**: ZendMM hands out chunks from its own arenas, so ASan and
  valgrind can't see request-memory errors without it. Set it for every detector run.
- **ASan, as CI runs it** (job "Test with ASan", Linux, nightly):

  ```sh
  RUSTFLAGS="-Zsanitizer=address -Clink-arg=-Wl,-undefined,dynamic_lookup" \
  USE_ZEND_ALLOC=0 USE_TRACKED_ALLOC=1 \
  LSAN_OPTIONS=suppressions=$PWD/.lsan-suppressions.txt \
  cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu \
    -p ext-php-rs --lib --test module_tests --features embed,closure,anyhow,observer
  ```

  `nix develop .#asan` sets these variables and provides an ASan-instrumented
  debug PHP with embed. This job covers the library's unit tests and
  `module_tests`, not class, enum or INI registration (MINIT can't run inside Embed's
  request): for those, load the extension into that ASan PHP as a cdylib.
- **LSan suppressions** are only for leaks that are by design (PHP's own persistent
  allocations at embed shutdown, tests that bail out on purpose). A real leak is fixed,
  never suppressed.
- macOS: ASan works for Rust code (`-Zsanitizer=address` on nightly), but there is no
  instrumented PHP and no nix shell; run the detector jobs on Linux or rely on CI.
