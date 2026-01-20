# ext-php-rs

[![Crates.io](https://img.shields.io/crates/v/ext-php-rs)](https://lib.rs/ext-php-rs)
[![docs.rs](https://img.shields.io/docsrs/ext-php-rs/latest)](https://docs.rs/ext-php-rs)
[![Guide Workflow Status](https://img.shields.io/github/actions/workflow/status/extphprs/ext-php-rs/docs.yml?branch=master&label=guide)](https://ext-php.rs)
![CI Workflow Status](https://img.shields.io/github/actions/workflow/status/extphprs/ext-php-rs/build.yml?branch=master)
[![Coverage Status](https://coveralls.io/repos/github/extphprs/ext-php-rs/badge.svg)](https://coveralls.io/github/extphprs/ext-php-rs)
[![Discord](https://img.shields.io/discord/115233111977099271)](https://discord.gg/dphp)

Bindings and abstractions for the Zend API to build PHP extensions natively in
Rust.

- Documentation: <https://docs.rs/ext-php-rs>
- Guide: <https://ext-php.rs>

## Example

Export a simple function `function hello_world(string $name): string` to PHP:

```rust
#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::prelude::*;

/// Gives you a nice greeting!
///
/// @param string $name Your name.
///
/// @return string Nice greeting!
#[php_function]
pub fn hello_world(name: String) -> String {
    format!("Hello, {}!", name)
}

// Required to register the extension with PHP.
#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(hello_world))
}
```

Use [`cargo-php`] to build IDE stubs and install the extension:

```text
$ cargo install cargo-php --locked
  Installing cargo-php v0.1.0
$ cargo php stubs --stdout
  Compiling example-ext v0.1.0
  Finished dev [unoptimized + debuginfo] target(s) in 3.57s
<?php

// Stubs for example-ext

/**
 * Gives you a nice greeting!
 *
 * @param string $name Your name.
 *
 * @return string Nice greeting!
 */
function hello_world(string $name): string {}
$ cargo php install --release
  Compiling example-ext v0.1.0
  Finished release [optimized] target(s) in 1.68s
Are you sure you want to install the extension `example-ext`? yes
$ php -m
[PHP Modules]
// ...
example-ext
// ...
```

Calling the function from PHP:

```php
var_dump(hello_world("David")); // string(13) "Hello, David!"
```

For more examples read the library
[guide](https://ext-php.rs).

[`cargo-php`]: https://crates.io/crates/cargo-php

## Features

- **Easy to use:** The built-in macros can abstract away the need to interact
  with the Zend API, such as Rust-type function parameter abstracting away
  interacting with Zend values.
- **Lightweight:** You don't have to use the built-in helper macros. It's
  possible to write your own glue code around your own functions.
- **Extensible:** Implement `IntoZval` and `FromZval` for your own custom types,
  allowing the type to be used as function parameters and return types.

## Goals

Our main goal is to **make extension development easier.**

- Writing extensions in C can be tedious, and with the Zend APIs limited
  documentation can be intimidating.
- Rust's modern language features and feature-full standard library are big
  improvements on C.
- Abstracting away the raw Zend APIs allows extensions to be developed faster
  and with more confidence.
- Abstractions also allow us to support future (and potentially past) versions
  of PHP without significant changes to extension code.

## Documentation

The library guide can be read
[here](https://ext-php.rs).

The project is documented in-line, so viewing the `cargo` documentation is the
best resource at the moment. This can be viewed at [docs.rs].

## Requirements

- Linux, macOS or Windows-based operating system.
- PHP 8.1 or later.
  - No support is planned for earlier versions of PHP.
  - PHP versions, that no longer receive security updates, will no longer be
    supported. They might still work, but no guarantees are made.
  - See <https://www.php.net/supported-versions.php> for information on PHP
    supported versions and their end of life dates.
- Rust.
  - Currently, we maintain no guarantee of a MSRV, however lib.rs suggests Rust
    1.57 at the time of writing.
- Clang 5.0 or later.

### Alpine Linux (musl)

Building on Alpine Linux (musl libc) is supported thanks to the `runtime` feature flag.

```sh
# Install dependencies and cargo-php
apk add clang16-libclang
cargo install cargo-php
```

When building your own extensions, add to your project's `.cargo/config.toml`:

```toml
[target.'cfg(target_env = "musl")']
rustflags = ["-C", "target-feature=-crt-static"]
```

For static linking, see: <https://github.com/KyleMayes/clang-sys#static>

### Windows Requirements

- Extensions can only be compiled for PHP installations sourced from
  <https://windows.php.net>. Support is planned for other installations
  eventually.
- Rust nightly is required for Windows. This is due to the [vectorcall] calling
  convention being used by some PHP functions on Windows, which is only
  available as a nightly unstable feature in Rust.
- It is suggested to use the `rust-lld` linker to link your extension. The MSVC
  linker (`link.exe`) is supported however you may run into issues if the linker
  version is not supported by your PHP installation. You can use the `rust-lld`
  linker by creating a `.cargo\config.toml` file with the following content:
  ```toml
  # Replace target triple if you have a different architecture than x86_64
  [target.x86_64-pc-windows-msvc]
  linker = "rust-lld"
  ```
- The `cc` crate requires `cl.exe` to be present on your system. This is usually
  bundled with Microsoft Visual Studio.
- `cargo-php`'s stub generation feature does not work on Windows. Rewriting this
  functionality to be cross-platform is on the roadmap.
- To build the application in `DEBUG` mode on Windows,
  you must have a `PHP SDK` built with the `DEBUG` option enabled
  and specify the `PHP_LIB` to the folder containing the lib files.
  For example: set `PHP_LIB=C:\php-sdk\php-dev\vc16\x64\php-8.3.13-src\x64\Debug_TS`.

[vectorcall]: https://docs.microsoft.com/en-us/cpp/cpp/vectorcall?view=msvc-170

## Cargo Features

All features are disabled by default.

- `closure` - Enables the ability to return Rust closures to PHP. Creates a new
  class type, `RustClosure`.
- `anyhow` - Implements `Into<PhpException>` for `anyhow::Error`, allowing you
  to return anyhow results from PHP functions. Supports anyhow v1.x.
- `static` - Allows building the library against
  [statically linked clang](https://github.com/KyleMayes/clang-sys?tab=readme-ov-file#static),
  for example with [static-php-cli](https://static-php.dev/)

## Usage

Check out one of the example projects:

- [anonaddy-sequoia](https://gitlab.com/willbrowning/anonaddy-sequoia) - Sequoia
  encryption PHP extension.
- [opus-php](https://github.com/davidcole1340/opus-php) - Audio encoder for the
  Opus codec in PHP.
- [tomlrs-php](https://github.com/jphenow/tomlrs-php) - TOML data format parser.
- [php-scrypt](https://github.com/appwrite/php-scrypt) - PHP wrapper for the
  scrypt password hashing algorithm.
- [fluent-php](https://github.com/Ennexa/fluent-php) - PHP wrapper for Mozilla's Project Fluent i18n library.
- [php-rocksdb-rc](https://github.com/s00d/php-rocksdb-rc) - PHP wrapper for rocksdb library.

## Contributions

We welcome contributions to the project! Please see the [CONTRIBUTING.md](CONTRIBUTING.md) file
for more information on how to contribute.

## Development

### `allowed_bindings.rs`
This file contains the list of allowed bindings for the Zend API. It is acting
as a whitelist for the bindings that can be used in the library.

When updating this file you need to also update the `docsrs_bindings.rs` file.
To do this, run the following command from the root of the project:

```bash
tools/update_bindings.sh
```

This will build the bindings using docker, and then copy the `docsrs_bindings.rs`
file to the root of the repository.
This will ensure that the file is always built in the same environment.

Docker and buildx are required to run the script, so make sure you have
those [installed](https://docs.docker.com/engine/install/).

## Resources

- [PHP Internals Book](https://www.phpinternalsbook.com/)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE_APACHE] or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE_MIT] or <http://opensource.org/licenses/MIT>)

at your option.

[LICENSE_APACHE]: https://github.com/extphprs/ext-php-rs/blob/master/LICENSE_APACHE
[LICENSE_MIT]: https://github.com/extphprs/ext-php-rs/blob/master/LICENSE_MIT
[docs.rs]: https://docs.rs/ext-php-rs
