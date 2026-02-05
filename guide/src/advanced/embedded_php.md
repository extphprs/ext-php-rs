# Embedded PHP Execution

Extensions sometimes need to execute PHP code at runtime for setup tasks
like registering autoloaders, defining helper classes, or configuring error
handlers. The `php_eval` module lets you embed `.php` files into your
extension binary at compile time and execute them from Rust.

## Why Not `eval`?

This module uses `zend_compile_string` + `zend_execute` instead of
`zend_eval_string` because:

- `zend_eval_string` triggers security scanner false positives (eval-like
  semantics)
- Some hardened PHP builds disable eval-like functionality
- The embedded code is static bytes in the binary -- there is no injection risk

## Basic Usage

### 1. Write your PHP file

Create a normal `.php` file with full IDE support (syntax highlighting,
linting, static analysis):

```php
<?php
// php/setup.php

spl_autoload_register(function (string $class): void {
    $prefix = 'Acme\\Encryption\\';
    if (str_starts_with($class, $prefix)) {
        $relative = substr($class, strlen($prefix));
        $file = __DIR__ . '/src/' . str_replace('\\', '/', $relative) . '.php';
        if (file_exists($file)) {
            require $file;
        }
    }
});

function acme_encrypt_version(): string {
    return '1.0.0';
}
```

### 2. Embed and execute from Rust

Use `include_bytes!` or `include_str!` to embed the file at compile time,
then call `php_eval::execute()` from whatever lifecycle hook fits your target
SAPI. The function accepts any type that implements `AsRef<[u8]>`:

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::php_eval;

// Both forms work:
const SETUP: &[u8] = include_bytes!("../php/setup.php");
// const SETUP: &str = include_str!("../php/setup.php");

unsafe extern "C" fn on_request_start(
    _type: i32,
    _module_number: i32,
) -> i32 {
    if let Err(e) = php_eval::execute(SETUP) {
        eprintln!("Failed to run embedded PHP setup: {:?}", e);
    }
    0
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.request_startup_function(on_request_start)
}
```

## API Reference

### `php_eval::execute(code: impl AsRef<[u8]>) -> Result<(), PhpEvalError>`

Compiles and executes the given PHP source within the running PHP engine.

**Arguments:**

- `code` -- Raw PHP source, typically from `include_bytes!` or `include_str!`.
  Any type implementing `AsRef<[u8]>` is accepted (`&[u8]`, `&str`,
  `String`, `Vec<u8>`, etc.).

**Returns:**

- `Ok(())` on success.
- `Err(PhpEvalError::MissingOpenTag)` if the code does not start with
  `<?php` (case-insensitive).
- `Err(PhpEvalError::CompilationFailed)` if PHP cannot compile the code
  (syntax error).
- `Err(PhpEvalError::ExecutionFailed)` if the code throws an unhandled
  exception.
- `Err(PhpEvalError::Bailout)` if a PHP fatal error occurs.

### Input handling

The code **must** start with a `<?php` opening tag (case-insensitive).
The tag is stripped before compilation.

| Input | Handling |
|-------|----------|
| `<?php` opening tag | **Required** (case-insensitive), stripped before compilation |
| UTF-8 BOM (`0xEF 0xBB 0xBF`) | Stripped before compilation |
| Empty after tag (e.g. `<?php`) | Returns `Ok(())` immediately |
| No `<?php` tag (including empty input) | Returns `Err(MissingOpenTag)` |

## Lifecycle Hooks

The module does not prescribe *when* to run embedded PHP. The SAPI landscape
is fragmented -- FrankenPHP in worker mode does not trigger RINIT per request,
for example. Choose the hook that fits your target:

| Hook | Use case |
|------|----------|
| RINIT (`request_startup_function`) | Per-request setup (classic php-fpm / mod_php) |
| MINIT (`startup_function`) | One-time global setup |
| Custom SAPI callback | Worker-mode runtimes (FrankenPHP, RoadRunner) |

## Error Handling

Errors during embedded PHP execution should not crash the host process.
The recommended pattern is to log and continue:

```rust,ignore
if let Err(e) = php_eval::execute(SETUP_CODE) {
    match e {
        PhpEvalError::MissingOpenTag => {
            eprintln!("embedded PHP missing <?php open tag");
        }
        PhpEvalError::CompilationFailed => {
            eprintln!("embedded PHP syntax error");
        }
        PhpEvalError::ExecutionFailed => {
            eprintln!("embedded PHP threw an exception");
        }
        PhpEvalError::Bailout => {
            eprintln!("embedded PHP fatal error");
        }
    }
}
```

## How It Works

1. **Build time**: `include_bytes!` embeds the `.php` file contents into the
   extension binary as a `&[u8]` constant.

2. **Runtime**: `php_eval::execute()` strips the `<?php` tag and BOM, then
   calls two C wrapper functions in `wrapper.c`:
   - `ext_php_rs_zend_compile_string` -- compiles the source into an op_array.
     On PHP 8.2+ it passes `ZEND_COMPILE_POSITION_AFTER_OPEN_TAG` so the
     scanner starts directly in PHP mode. On PHP 8.1 the two-argument form
     is used.
   - `ext_php_rs_zend_execute` -- executes the op_array, sets the execution
     scope, then cleans up static vars and frees the op_array.

3. **Safety**: The entire execution is wrapped in `try_catch` to catch PHP
   bailouts (longjmp) without unwinding the Rust stack. Error reporting is
   suppressed during execution and restored afterward.
