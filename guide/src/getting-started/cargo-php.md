# `cargo php`

ext-php-rs comes with a cargo subcommand called [`cargo-php`]. When called in
the manifest directory of an extension, it allows you to do the following:

- Generate IDE stub files
- Install the extension
- Remove the extension

## System Requirements

The subcommand has been tested on the following systems and architectures. Note
these are not requirements, but simply platforms that the application have been
tested on. YMMV.

- macOS 12.0 (AArch64, x86_64 builds but untested)
- Linux 5.15.1 (AArch64, x86_64 builds but untested)

Windows is not currently supported by `ext-php-rs`.

### macOS Note

When installing your extension multiple times without uninstalling on macOS, you
may run into PHP exiting with `SIGKILL`. You can see the exact cause of the exit
in Console, however, generally this is due to a invalid code signature.
Uninstalling the extension and then reinstalling generally fixes this problem.

## Installation

The subcommand is installed through composer like any other Rust CLI
application:

```text
$ cargo install cargo-php --locked
```

You can then call the application via `cargo php` (assuming the cargo
installation directory is in your PATH):

```text
$ cargo php --help
cargo-php 0.1.0

David Cole <david.cole1340@gmail.com>

Installs extensions and generates stub files for PHP extensions generated with `ext-php-rs`.

USAGE:
    cargo-php <SUBCOMMAND>

OPTIONS:
    -h, --help
            Print help information

    -V, --version
            Print version information

SUBCOMMANDS:
    help
            Print this message or the help of the given subcommand(s)
    install
            Installs the extension in the current PHP installation
    remove
            Removes the extension in the current PHP installation
    stubs
            Generates stub PHP files for the extension
```

The command should always be executed from within your extensions manifest
directory (the directory with your `Cargo.toml`).

## Stubs

Stub files are used by your IDEs language server to know the signature of
methods, classes and constants in your PHP extension, similar to how a C header
file works.

One of the largest collection of PHP standard library and non-standard extension
stub files is provided by JetBrains: [phpstorm-stubs]. This collection is used
by JetBrains PhpStorm and the PHP Intelephense language server (which I
personally recommend for use in Visual Studio Code).

### Usage

```text
$ cargo php stubs --help
cargo-php-stubs

Generates stub PHP files for the extension.

These stub files can be used in IDEs to provide typehinting for extension classes, functions and
constants.

USAGE:
    cargo-php stubs [OPTIONS] [EXT]

ARGS:
    <EXT>
            Path to extension to generate stubs for. Defaults for searching the directory the
            executable is located in

OPTIONS:
    -h, --help
            Print help information

        --manifest <MANIFEST>
            Path to the Cargo manifest of the extension. Defaults to the manifest in the directory
            the command is called.

            This cannot be provided alongside the `ext` option, as that option provides a direct
            path to the extension shared library.

    -o, --out <OUT>
            Path used to store generated stub file. Defaults to writing to `<ext-name>.stubs.php` in
            the current directory

        --stdout
            Print stubs to stdout rather than write to file. Cannot be used with `out`
```

### Custom Allocators

If your extension uses a custom global allocator (e.g., `mimalloc`, `jemalloc`),
you may encounter a crash when running `cargo php stubs` with an error like
"pointer being freed was not allocated".

This happens because `cargo php stubs` builds your extension in debug mode and
loads it to extract type information. Custom allocators can conflict with this
process.

**Workaround:** Conditionally enable your custom allocator only in release builds:

```rust,ignore
#[cfg(not(debug_assertions))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

See [issue #523](https://github.com/extphprs/ext-php-rs/issues/523) for more
details.

## Extension Installation

When PHP is in your PATH, the application can automatically build and copy your
extension into PHP. This requires `php-config` to be installed alongside PHP.

It is recommended to backup your `php.ini` **before** installing the extension
so you are able to restore if you run into any issues.

### Usage

```text
$ cargo php install --help
cargo-php-install

Installs the extension in the current PHP installation.

This copies the extension to the PHP installation and adds the extension to a PHP configuration
file.

Note that this uses the `php-config` executable installed alongside PHP to locate your `php.ini`
file and extension directory. If you want to use a different `php-config`, the application will read
the `PHP_CONFIG` variable (if it is set), and will use this as the path to the executable instead.

USAGE:
    cargo-php install [OPTIONS]

OPTIONS:
        --disable
            Installs the extension but doesn't enable the extension in the `php.ini` file

    -h, --help
            Print help information

        --ini-path <INI_PATH>
            Path to the `php.ini` file to update with the new extension

        --install-dir <INSTALL_DIR>
            Changes the path that the extension is copied to. This will not activate the extension
            unless `ini_path` is also passed

        --manifest <MANIFEST>
            Path to the Cargo manifest of the extension. Defaults to the manifest in the directory
            the command is called

        --release
            Whether to install the release version of the extension

        --yes
            Bypasses the confirmation prompt
```

## Extension Removal

Removes the extension from your PHPs extension directory, and removes the entry
from your `php.ini` if present.

### Usage

```text
$ cargo php remove --help
cargo-php-remove

Removes the extension in the current PHP installation.

This deletes the extension from the PHP installation and also removes it from the main PHP
configuration file.

Note that this uses the `php-config` executable installed alongside PHP to locate your `php.ini`
file and extension directory. If you want to use a different `php-config`, the application will read
the `PHP_CONFIG` variable (if it is set), and will use this as the path to the executable instead.

USAGE:
    cargo-php remove [OPTIONS]

OPTIONS:
    -h, --help
            Print help information

        --ini-path <INI_PATH>
            Path to the `php.ini` file to remove the extension from

        --install-dir <INSTALL_DIR>
            Changes the path that the extension will be removed from. This will not remove the
            extension from a configuration file unless `ini_path` is also passed

        --manifest <MANIFEST>
            Path to the Cargo manifest of the extension. Defaults to the manifest in the directory
            the command is called

        --yes
            Bypasses the confirmation prompt
```

[`cargo-php`]: https://crates.io/crates/cargo-php
[phpstorm-stubs]: https://github.com/JetBrains/phpstorm-stubs#readme
