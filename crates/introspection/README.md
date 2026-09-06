# ext-php-rs-introspection

ABI-stable description of an [`ext-php-rs`](https://github.com/extphprs/ext-php-rs)
extension, and the PHP stub rendering built on top of it.

An extension exports `ext_php_rs_describe_module`, which returns a
`Description`: the module name, its functions, classes, enums and constants as
`#[repr(C)]` types. `cargo php stubs` loads the extension with `dlopen`, calls
that function and renders the result with `ToStub`.

This crate has no dependency on the Zend engine. `ext-php-rs` re-exports it as
`ext_php_rs::describe` and fills the types from its builders; `cargo-php`
depends on it alone, so the CLI builds without PHP installed.

## Versioning

`Description::version` carries this crate's version. Any change to a
`#[repr(C)]` type is a breaking release, and `cargo-php` refuses an extension
whose version is not semver compatible with the one it was built against.

## License

MIT OR Apache-2.0, like the rest of the workspace.
