# Static Linking into php-src

Extensions built with `ext-php-rs` are normally compiled as a `cdylib` and
loaded at runtime with an `extension=` ini line. PHP also supports compiling
extensions directly into the `php` binary. This is how fully static PHP
builds work, for example [FrankenPHP](https://frankenphp.dev/) static binaries
built with [static-php-cli](https://github.com/crazywhalecc/static-php-cli):
there is no `.so` and no ini line, the extension is part of the binary.

This page describes how to build an `ext-php-rs` extension as a static library
and link it into php-src. Note that this is an advanced use-case; if in doubt
prefer loading your extension(s) dynamically and make sure to read the
Caveats section below if you go the static route.

## How php-src registers builtin extensions

Every builtin extension lives in `php-src/ext/<name>/` with a `config.m4`.
During `./buildconf`, php-src scans the headers of enabled extensions for the
token `phpext_` and collects `phpext_<name>_ptr` pointers into the
`php_builtin_extensions[]` array in `main/internal_functions.c`. That array
is registered during `php_module_startup()`.

The array needs a link-time-constant address, conventionally
`&<name>_module_entry`. An `ext-php-rs` extension builds its
`zend_module_entry` at runtime inside `get_module()`, so a small C shim
bridges the two: it defines the `<name>_module_entry` symbol and fills it
from `get_module()` in a constructor, which runs before
`php_module_startup()`.

## Crate setup

Add `staticlib` to your crate type, keeping `cdylib` for regular builds:

```toml
[lib]
crate-type = ["staticlib", "cdylib"]
```

For thread-safe (ZTS) targets, the extension must be compiled with
`EXT_PHP_RS_STATIC_TSRMLS_CACHE=1`:

```sh
EXT_PHP_RS_STATIC_TSRMLS_CACHE=1 cargo build --release
```

This defines `ZEND_ENABLE_STATIC_TSRMLS_CACHE`, which makes the extension
read the main binary's `_tsrm_ls_cache` thread-local directly instead of
calling `tsrm_get_ls_cache()` on every globals access. php-src compiles all
builtin extensions this way. The variable is harmless on non-ZTS targets.

Note that a static library built in this mode can only be linked into a PHP
binary. It cannot be loaded with `extension=` or linked against a prebuilt
`libphp`, because `_tsrm_ls_cache` only resolves inside php-src itself.

## Generating the glue

The [`cargo-php`](https://crates.io/crates/cargo-php) CLI generates the three
files php-src needs:

```sh
cargo php static-glue
```

This writes a directory named after your extension containing:

- `config.m4`: enables the extension with `--enable-<name>` and links the
  prebuilt `lib<name>.a`.
- `php_<name>.h`: declares `<name>_module_entry` and the
  `phpext_<name>_ptr` define that php-src scans for.
- `<name>_glue.c`: the constructor shim copying `*get_module()` into
  `<name>_module_entry`.

The extension name defaults to the library target name with dashes replaced
by underscores; override it with `--ext-name`.

## The two-pass build

Building the Rust library requires `php-config` and the PHP headers, which do
not exist yet when you are building the very PHP you want to link into. The
solution is two passes:

1. Build (or install) a PHP with the exact same version, ZTS mode and debug
   mode as your final binary. Only its `php-config` and headers are needed.

   ```sh
   cd php-src
   ./buildconf
   ./configure --enable-zts --disable-all --enable-cli --prefix=$HOME/php-pass1
   make -j$(nproc) && make install
   ```

2. Build the Rust static library against pass 1:

   ```sh
   PHP_CONFIG=$HOME/php-pass1/bin/php-config \
   EXT_PHP_RS_STATIC_TSRMLS_CACHE=1 \
   cargo build --release
   ```

3. Copy the glue and the static library into php-src:

   ```sh
   cargo php static-glue
   cp -r my_ext php-src/ext/my_ext
   cp target/release/libmy_ext.a php-src/ext/my_ext/
   ```

4. Build the final PHP:

   ```sh
   cd php-src
   ./buildconf --force
   ./configure --enable-zts --enable-my_ext <other flags>
   make -j$(nproc)
   ```

   Alternatively, `./config.nice --enable-my_ext` reruns `configure` with the
   flags of the previous invocation plus the new one.

5. Verify:

   ```sh
   sapi/cli/php -m | grep my_ext
   sapi/cli/php -r 'var_dump(my_function());'
   ```

The module is listed without any `extension=` ini line. Note that `php -m`
prints the module name from your crate's `Cargo.toml` package name, which can
differ from the C extension name when the package name contains dashes.

For FrankenPHP and static-php-cli the integration point is identical: drop
the glue directory and the `.a` into the php-src tree those tools build from
and add `--enable-<name>` to the configure flags. ZTS is mandatory there, so
`EXT_PHP_RS_STATIC_TSRMLS_CACHE=1` is required.

## Caveats

- **One Rust extension per PHP binary.** The `get_module` and `ext_php_rs_*`
  symbols have fixed names, and two Rust static libraries collide on the Rust
  standard library symbols. Bundle all your Rust functionality into a single
  extension crate.
- **gcc/clang only.** The shim relies on `__attribute__((constructor))`.
  Statically linking into a Windows PHP build is not supported.
- **Keep the `.a` in sync.** The static library embeds bindings generated
  from the pass-1 PHP headers. Rebuild it whenever the target PHP version
  changes.
- `--enable-<name>=shared` is rejected by the generated `config.m4`: for
  shared builds, use the regular `cdylib` workflow instead.
