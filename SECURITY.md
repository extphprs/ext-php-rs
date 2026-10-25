# Security Policy

## Supported Versions

We release security fixes only for the latest `0.x` release line on crates.io, for example `0.16.x`.
We do not release fixes for older release lines.
This policy applies to all the crates in this repository:

- `ext-php-rs`
- `ext-php-rs-derive`
- `ext-php-rs-build`
- `ext-php-rs-introspection`
- `cargo-php`

We do not fix issues that occur only on PHP versions that are at end of life.

## Report a Vulnerability

Do not open a public issue, discussion, or pull request for a security problem.

Send the report through
[GitHub private vulnerability reporting](https://github.com/extphprs/ext-php-rs/security/advisories/new).

Give this data in the report:

- The crate and the version
- The PHP version and the build type (ZTS or NTS)
- The operating system
- A small example that shows the problem
- The effect of the problem, for example a crash or a memory corruption

## Scope

These problems are security problems:

- Undefined behavior that safe Rust code can cause through the `ext-php-rs` API
- Memory errors, for example use after free, double free, or out-of-bounds access
- Unsound code that the macros generate
- PHP code that can corrupt memory in an extension that uses only the safe API

These problems are not in the scope of this policy:

- Problems that occur only when `unsafe` code does not obey its documented safety contract
- Vulnerabilities in PHP itself. Report them to [php-src](https://github.com/php/php-src/security).
- Vulnerabilities in an extension that uses `ext-php-rs`. Report them to the authors of that extension.

## Our Process

One volunteer maintains this project. The time below is a target, not a guarantee.

1. We reply to your report in 30 days or less.
2. We find the cause and make a fix in a private fork.
3. We release the fix and publish a GitHub security advisory.
4. We send the advisory to the [RustSec Advisory Database](https://github.com/rustsec/advisory-db).

We put your name in the advisory, unless you tell us not to.
