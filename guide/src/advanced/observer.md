# Observer API

The Observer API allows you to build profilers, tracers, and instrumentation tools
that observe PHP function calls. This is useful for:

- Performance profiling
- Request tracing (APM)
- Code coverage tools
- Debugging tools

## Enabling the Feature

The Observer API is behind a feature flag. Add it to your `Cargo.toml`:

```toml
[dependencies]
ext-php-rs = { version = "0.15", features = ["observer"] }
```

## Basic Usage

Implement the `FcallObserver` trait to create an observer:

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;
use std::sync::atomic::{AtomicU64, Ordering};

/// A simple profiler that counts function calls.
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
        // Only observe user-defined functions (not internal PHP functions)
        !info.is_internal
    }

    fn begin(&self, _execute_data: &ExecuteData) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn end(&self, _execute_data: &ExecuteData, _retval: Option<&Zval>) {
        // Called when the function returns
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.fcall_observer(CallCounter::new)
}
```

## The `FcallObserver` Trait

The trait has three methods:

### `should_observe`

```rust,ignore
fn should_observe(&self, info: &FcallInfo) -> bool;
```

Called once per unique function to determine if it should be observed.
The result is cached by PHP, so this is only called once per function definition.

Return `true` to observe the function, `false` to skip it.

### `begin`

```rust,ignore
fn begin(&self, execute_data: &ExecuteData);
```

Called when an observed function begins execution. Use this to:
- Record start timestamps for profiling
- Push to a call stack for tracing
- Log function entry

### `end`

```rust,ignore
fn end(&self, execute_data: &ExecuteData, retval: Option<&Zval>);
```

Called when an observed function ends execution. This is called even if
the function throws an exception. Use this to:
- Calculate execution duration
- Pop from a call stack
- Record return values

## `FcallInfo` - Function Metadata

The `FcallInfo` struct provides metadata about the function being called:

| Field | Type | Description |
|-------|------|-------------|
| `function_name` | `Option<&str>` | Function name (None for anonymous/main) |
| `class_name` | `Option<&str>` | Class name for methods |
| `filename` | `Option<&str>` | Source file (None for internal functions) |
| `lineno` | `u32` | Line number (0 for internal functions) |
| `is_internal` | `bool` | True for built-in PHP functions |

## Thread Safety

The observer is created once during MINIT and stored as a global singleton
shared across all requests. The observer must implement `Send + Sync` because:

- **NTS (Non-Thread-Safe)**: A single global observer instance handles all requests
- **ZTS (Thread-Safe)**: The same observer instance may be called from different threads

Use thread-safe primitives like `AtomicU64`, `Mutex`, or `RwLock` for
any mutable state in your observer.

## Example: Simple Profiler

Here's a more complete example that tracks function timing:

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;
use std::sync::Mutex;
use std::time::Instant;
use std::collections::HashMap;

struct TimingProfiler {
    // Track start times per call (thread-safe)
    call_stack: Mutex<Vec<(String, Instant)>>,
    // Accumulated timings
    timings: Mutex<HashMap<String, u128>>,
}

impl TimingProfiler {
    fn new() -> Self {
        Self {
            call_stack: Mutex::new(Vec::new()),
            timings: Mutex::new(HashMap::new()),
        }
    }
}

impl FcallObserver for TimingProfiler {
    fn should_observe(&self, info: &FcallInfo) -> bool {
        // Only observe user functions
        !info.is_internal
    }

    fn begin(&self, execute_data: &ExecuteData) {
        let func_name = execute_data
            .get_function_name()
            .unwrap_or_else(|| "unknown".to_string());

        if let Ok(mut stack) = self.call_stack.lock() {
            stack.push((func_name, Instant::now()));
        }
    }

    fn end(&self, _execute_data: &ExecuteData, _retval: Option<&Zval>) {
        if let Ok(mut stack) = self.call_stack.lock() {
            if let Some((func_name, start)) = stack.pop() {
                let duration = start.elapsed().as_micros();

                if let Ok(mut timings) = self.timings.lock() {
                    *timings.entry(func_name).or_insert(0) += duration;
                }
            }
        }
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.fcall_observer(TimingProfiler::new)
}
```

## Best Practices

1. **Keep observers lightweight**: The `begin` and `end` methods are called
   for every function invocation. Avoid heavy computations or I/O.

2. **Use `should_observe` wisely**: Filter out functions you don't need
   to observe. This is cached by PHP, so complex logic here is acceptable.

3. **Handle errors gracefully**: Don't panic in observer methods. Use
   `Result` types and log errors instead.

4. **Consider memory usage**: If storing call data, implement limits or
   periodic flushing to avoid unbounded memory growth.

5. **Test with real workloads**: Profile your observer itself to ensure
   it doesn't add significant overhead.

## Limitations

- Only function calls can be observed (not opcodes or other events)
- The observer is registered globally for the entire PHP process
- Only one observer can be registered per extension
