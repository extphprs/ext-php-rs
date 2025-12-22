# Output

`ext-php-rs` provides several macros and functions for writing output to PHP's
stdout and stderr streams. These are essential when your extension needs to
produce output that integrates with PHP's output buffering system.

## Text Output

For regular text output (strings without NUL bytes), use the `php_print!` and
`php_println!` macros. These work similarly to Rust's `print!` and `println!`
macros.

### `php_print!`

Prints to PHP's standard output without a trailing newline.

```rust,ignore
use ext_php_rs::prelude::*;

#[php_function]
pub fn greet(name: &str) {
    php_print!("Hello, {}!", name);
}
```

### `php_println!`

Prints to PHP's standard output with a trailing newline.

```rust,ignore
use ext_php_rs::prelude::*;

#[php_function]
pub fn greet(name: &str) {
    php_println!("Hello, {}!", name);
}
```

> **Note:** `php_print!` and `php_println!` will panic if the string contains
> NUL bytes (`\0`). For binary-safe output, use `php_output!` or `php_write!`.

## Binary-Safe Output

When working with binary data that may contain NUL bytes, use the binary-safe
output functions. These are essential for outputting raw bytes, binary file
contents, or any data that might contain `\0` characters.

### `php_output!`

Writes binary data to PHP's output stream. This macro is **both binary-safe AND
respects PHP's output buffering** (`ob_start()`). This is usually what you want
for binary output.

```rust,ignore
use ext_php_rs::prelude::*;

#[php_function]
pub fn output_binary() -> i64 {
    // Write binary data with NUL bytes - will be captured by ob_start()
    let bytes_written = php_output!(b"Hello\x00World");
    bytes_written as i64
}
```

### `php_write!`

Writes binary data directly to the SAPI output, **bypassing PHP's output
buffering**. This macro is binary-safe but output will NOT be captured by
`ob_start()`. The "ub" in `ub_write` stands for "unbuffered".

```rust,ignore
use ext_php_rs::prelude::*;

#[php_function]
pub fn output_binary() -> i64 {
    // Write a byte literal
    php_write!(b"Hello World").expect("write failed");

    // Write binary data with NUL bytes (would panic with php_print!)
    let bytes_written = php_write!(b"Hello\x00World").expect("write failed");

    // Write a byte slice
    let data: &[u8] = &[0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
    php_write!(data).expect("write failed");

    bytes_written as i64
}
```

The macro returns a `Result<usize>` with the number of bytes written, which can
be useful for verifying that all data was output successfully. The error case
occurs when the SAPI's `ub_write` function is not available.

## Function API

In addition to macros, you can use the underlying functions directly:

| Function | Binary-Safe | Output Buffering | Description |
|----------|-------------|------------------|-------------|
| `zend::printf()` | No | Yes | Printf-style output (used by `php_print!`) |
| `zend::output_write()` | Yes | Yes | Binary-safe buffered output |
| `zend::write()` | Yes | No | Binary-safe unbuffered output |

### Example using functions directly

```rust,ignore
use ext_php_rs::zend::output_write;

fn output_data(data: &[u8]) {
    let bytes_written = output_write(data);
    if bytes_written != data.len() {
        eprintln!("Warning: incomplete write");
    }
}
```

## Comparison

| Macro | Binary-Safe | Output Buffering | Supports Formatting |
|-------|-------------|------------------|---------------------|
| `php_print!` | No | Yes | Yes |
| `php_println!` | No | Yes | Yes |
| `php_output!` | Yes | Yes | No |
| `php_write!` | Yes | No | No |

## When to Use Each

- **`php_print!` / `php_println!`**: Use for text output with format strings,
  similar to Rust's `print!` and `println!`. Best for human-readable messages.

- **`php_output!`**: Use for binary data that needs to work with PHP's output
  buffering. This is the recommended choice for most binary output needs.

- **`php_write!`**: Use when you need direct, unbuffered output that bypasses
  PHP's output layer. Useful for low-level SAPI interaction or when output
  buffering must be avoided.
