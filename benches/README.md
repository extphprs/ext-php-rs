# Benchmarks

Benchmarks spawn `php -dextension=ext/target/release/libbenches.so <script> <count>`
for each PHP script in `benches/` and measure the whole PHP process. Results are
tracked on [CodSpeed](https://codspeed.io/extphprs/ext-php-rs) by the
`Benchmarks` workflow in `simulation` mode (instruction count, php child tracked
through `simulation-track-subprocess`).

The harness is [divan](https://docs.rs/divan) through
[`codspeed-divan-compat`](https://codspeed.io/docs/benchmarks/rust/divan), so
plain `cargo bench` keeps working as a walltime run.

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
