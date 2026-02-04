# Observer API

The Observer API allows you to build profilers, tracers, and instrumentation tools
that observe PHP function calls and errors. This is useful for:

- Performance profiling
- Request tracing (APM)
- Error monitoring
- Code coverage tools
- Debugging tools

## Enabling the Feature

The Observer API is behind a feature flag. Add it to your `Cargo.toml`:

```toml
[dependencies]
ext-php-rs = { version = "0.15", features = ["observer"] }
```

## Function Call Observer

Implement the `FcallObserver` trait to observe function calls:

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;
use std::sync::atomic::{AtomicU64, Ordering};

struct CallCounter {
    count: AtomicU64,
}

impl CallCounter {
    fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
        }
    }
}

impl FcallObserver for CallCounter {
    fn should_observe(&self, info: &FcallInfo) -> bool {
        !info.is_internal
    }

    fn begin(&self, _execute_data: &ExecuteData) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn end(&self, _execute_data: &ExecuteData, _retval: Option<&Zval>) {}
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.fcall_observer(CallCounter::new)
}
```

### The `FcallObserver` Trait

| Method | Description |
|--------|-------------|
| `should_observe(&self, info: &FcallInfo) -> bool` | Called once per function definition. Result is cached by PHP. |
| `begin(&self, execute_data: &ExecuteData)` | Called when function begins execution. |
| `end(&self, execute_data: &ExecuteData, retval: Option<&Zval>)` | Called when function ends (even on exceptions). |

### `FcallInfo` - Function Metadata

| Field | Type | Description |
|-------|------|-------------|
| `function_name` | `Option<&str>` | Function name (None for anonymous/main) |
| `class_name` | `Option<&str>` | Class name for methods |
| `filename` | `Option<&str>` | Source file (None for internal functions) |
| `lineno` | `u32` | Line number (0 for internal functions) |
| `is_internal` | `bool` | True for built-in PHP functions |

## Error Observer

Implement the `ErrorObserver` trait to observe PHP errors:

```rust,ignore
use ext_php_rs::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct ErrorTracker {
    fatal_count: AtomicU64,
    warning_count: AtomicU64,
}

impl ErrorTracker {
    fn new() -> Self {
        Self {
            fatal_count: AtomicU64::new(0),
            warning_count: AtomicU64::new(0),
        }
    }
}

impl ErrorObserver for ErrorTracker {
    fn should_observe(&self, error_type: ErrorType) -> bool {
        (ErrorType::FATAL | ErrorType::WARNING).contains(error_type)
    }

    fn on_error(&self, error: &ErrorInfo) {
        if ErrorType::FATAL.contains(error.error_type) {
            self.fatal_count.fetch_add(1, Ordering::Relaxed);

            if let Some(trace) = error.backtrace() {
                for frame in trace {
                    eprintln!("  at {}:{}",
                        frame.file.as_deref().unwrap_or("<internal>"),
                        frame.line
                    );
                }
            }
        } else {
            self.warning_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.error_observer(ErrorTracker::new)
}
```

### The `ErrorObserver` Trait

| Method | Description |
|--------|-------------|
| `should_observe(&self, error_type: ErrorType) -> bool` | Filter which error types to observe. |
| `on_error(&self, error: &ErrorInfo)` | Called when an observed error occurs. |

### `ErrorType` - Error Level Bitflags

```rust,ignore
ErrorType::ERROR           // E_ERROR
ErrorType::WARNING         // E_WARNING
ErrorType::PARSE           // E_PARSE
ErrorType::NOTICE          // E_NOTICE
ErrorType::CORE_ERROR      // E_CORE_ERROR
ErrorType::CORE_WARNING    // E_CORE_WARNING
ErrorType::COMPILE_ERROR   // E_COMPILE_ERROR
ErrorType::COMPILE_WARNING // E_COMPILE_WARNING
ErrorType::USER_ERROR      // E_USER_ERROR
ErrorType::USER_WARNING    // E_USER_WARNING
ErrorType::USER_NOTICE     // E_USER_NOTICE
ErrorType::RECOVERABLE_ERROR // E_RECOVERABLE_ERROR
ErrorType::DEPRECATED      // E_DEPRECATED
ErrorType::USER_DEPRECATED // E_USER_DEPRECATED

// Convenience groups
ErrorType::ALL   // All error types
ErrorType::FATAL // ERROR | CORE_ERROR | COMPILE_ERROR | USER_ERROR | RECOVERABLE_ERROR | PARSE
ErrorType::CORE  // CORE_ERROR | CORE_WARNING
```

### `ErrorInfo` - Error Metadata

| Field | Type | Description |
|-------|------|-------------|
| `error_type` | `ErrorType` | The error level/severity |
| `filename` | `Option<&str>` | Source file where error occurred |
| `lineno` | `u32` | Line number |
| `message` | `&str` | The error message |

### Lazy Backtrace

The `backtrace()` method captures the PHP call stack on demand:

```rust,ignore
fn on_error(&self, error: &ErrorInfo) {
    if let Some(trace) = error.backtrace() {
        for frame in trace {
            println!("{}::{}() at {}:{}",
                frame.class.as_deref().unwrap_or(""),
                frame.function.as_deref().unwrap_or("<main>"),
                frame.file.as_deref().unwrap_or("<internal>"),
                frame.line
            );
        }
    }
}
```

The backtrace is only captured when called, so there's zero cost if unused.

## Exception Observer

Implement the `ExceptionObserver` trait to observe thrown PHP exceptions:

```rust,ignore
use ext_php_rs::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct ExceptionTracker {
    exception_count: AtomicU64,
}

impl ExceptionTracker {
    fn new() -> Self {
        Self {
            exception_count: AtomicU64::new(0),
        }
    }
}

impl ExceptionObserver for ExceptionTracker {
    fn on_exception(&self, exception: &ExceptionInfo) {
        self.exception_count.fetch_add(1, Ordering::Relaxed);
        eprintln!("[EXCEPTION] {}: {} at {}:{}",
            exception.class_name,
            exception.message.as_deref().unwrap_or("<no message>"),
            exception.file.as_deref().unwrap_or("<unknown>"),
            exception.line
        );
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.exception_observer(ExceptionTracker::new)
}
```

### The `ExceptionObserver` Trait

| Method | Description |
|--------|-------------|
| `on_exception(&self, exception: &ExceptionInfo)` | Called when an exception is thrown, before any catch blocks. |

### `ExceptionInfo` - Exception Metadata

| Field | Type | Description |
|-------|------|-------------|
| `class_name` | `String` | Exception class name (e.g., "RuntimeException") |
| `message` | `Option<String>` | The exception message |
| `code` | `i64` | The exception code |
| `file` | `Option<String>` | Source file where thrown |
| `line` | `u32` | Line number where thrown |

### Exception Backtrace

The `backtrace()` method captures the PHP call stack at exception throw time:

```rust,ignore
impl ExceptionObserver for MyObserver {
    fn on_exception(&self, exception: &ExceptionInfo) {
        eprintln!("[EXCEPTION] {}: {}",
            exception.class_name,
            exception.message.as_deref().unwrap_or("<no message>")
        );

        if let Some(trace) = exception.backtrace() {
            for frame in trace {
                eprintln!("  at {}::{}() in {}:{}",
                    frame.class.as_deref().unwrap_or(""),
                    frame.function.as_deref().unwrap_or("<main>"),
                    frame.file.as_deref().unwrap_or("<internal>"),
                    frame.line
                );
            }
        }
    }
}
```

The backtrace is lazy - only captured when called, so there's zero cost if unused.

### `BacktraceFrame` - Stack Frame Metadata

| Field | Type | Description |
|-------|------|-------------|
| `function` | `Option<String>` | Function name (None for main script) |
| `class` | `Option<String>` | Class name for method calls |
| `file` | `Option<String>` | Source file |
| `line` | `u32` | Line number |

## Using All Observers

You can register all observers on the same module:

```rust,ignore
#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .fcall_observer(MyProfiler::new)
        .error_observer(MyErrorTracker::new)
        .exception_observer(MyExceptionTracker::new)
}
```

## Thread Safety

Observers are created once during MINIT and stored as global singletons.
They must implement `Send + Sync` because:

- **NTS**: A single instance handles all requests
- **ZTS**: The same instance may be called from different threads

Use thread-safe primitives like `AtomicU64`, `Mutex`, or `RwLock` for mutable state.

## Best Practices

1. **Keep observers lightweight**: Observer methods are called frequently.
   Avoid heavy computations or I/O.

2. **Use filtering wisely**: `should_observe` results are cached for fcall observers.
   For error observers, filter early to avoid unnecessary processing.

3. **Handle errors gracefully**: Don't panic in observer methods.

4. **Consider memory usage**: Implement limits or periodic flushing to avoid
   unbounded memory growth.

5. **Use lazy backtrace**: Only call `backtrace()` when needed. Both `ErrorInfo`
   and `ExceptionInfo` support lazy backtrace capture.

## Limitations

- Only one fcall observer can be registered per extension
- Only one error observer can be registered per extension
- Only one exception observer can be registered per extension
- Observers are registered globally for the entire PHP process
