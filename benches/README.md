# Benchmarks

Benchmarks spawn `php -n -dextension=ext/target/release/libbenches.so <script> <count>`
for each PHP script in `benches/` and measure the whole PHP process. With `-n`,
PHP does not read `php.ini` and does not load the shared extensions of the host.
Thus, most of the measured instructions come from the extension. Results are
tracked on [CodSpeed](https://codspeed.io/extphprs/ext-php-rs) by the
`Benchmarks` workflow in `simulation` mode (instruction count, php child tracked
through `simulation-track-subprocess`).

The harness is [divan](https://docs.rs/divan) through
[`codspeed-divan-compat`](https://codspeed.io/docs/benchmarks/rust/divan), so
plain `cargo bench` keeps working as a walltime run.

## Benchmarks

Each script in `benches/` calls the extension `<count>` times:

| Script | Measured path |
| --- | --- |
| `function_call.php` | Call of a `#[php_function]` with an integer |
| `method_call.php` | Call of an instance method |
| `static_method_call.php` | Call of a static method |
| `callback_call.php` | Call of a PHP closure from Rust |
| `string_call.php` | `&str` argument and `String` return value |
| `binary_slice.php` | `BinarySlice<u64>` argument |
| `object_new.php` | `new` on a `#[php_class]` with a constructor |
| `exception_throw.php` | `Err` from Rust, caught as `Exception` in PHP |
| `output_passthrough.php` | Output handler that passes 1 KiB through on `ob_flush()` |
| `output_transform.php` | Output handler that returns a new 1 KiB buffer on `ob_flush()` |
| `property_read.php` | Read of Rust properties and a getter |
| `property_write.php` | Write of Rust properties and a setter |
| `property_compound.php` | `+=`, `++` and `.=` on Rust properties |
| `property_compare.php` | `<=>` on two objects with Rust properties |
| `property_dump.php` | `var_dump` of an object with Rust properties |
| `array_str_ref_keys.php` | Array insert with `&str` keys |
| `array_interned_keys.php` | Array insert with interned keys |

## Running locally

Always from a nix dev shell, which provides `php` and `cargo-codspeed`:

```sh
cd benches
nix develop -c cargo bench
nix develop -c cargo codspeed build
nix develop -c cargo codspeed run
```

Without the CodSpeed runner, `cargo codspeed run` only prints `Checked: ...` for
every benchmark, which is enough to validate the suite.

The bench binary builds `ext/` (the PHP extension under test) with the features
it was itself compiled with before running. That build is skipped when
`CODSPEED_ENV` is set: under the CodSpeed runner every child process runs inside
valgrind, so CI builds the extension in a separate step first.
