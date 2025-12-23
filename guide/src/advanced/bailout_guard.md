# Bailout Guard

When PHP triggers a "bailout" (via `exit()`, `die()`, or a fatal error), it uses
`longjmp` to unwind the stack. This bypasses Rust's normal drop semantics,
meaning destructors for stack-allocated values won't run. This can lead to
resource leaks for things like file handles, network connections, or locks.

## The Problem

Consider this code:

```rust,ignore
#[php_function]
pub fn process_file(callback: ZendCallable) {
    let file = File::open("data.txt").unwrap();

    // If callback calls exit(), the file handle leaks!
    callback.try_call(vec![]);

    // file.drop() never runs
}
```

If the PHP callback triggers `exit()`, the `File` handle is never closed because
`longjmp` skips Rust's destructor calls.

## Solution 1: Using `try_call`

The simplest solution is to use `try_call` for PHP callbacks. It catches bailouts
internally and returns normally, allowing Rust destructors to run:

```rust,ignore
#[php_function]
pub fn process_file(callback: ZendCallable) {
    let file = File::open("data.txt").unwrap();

    // try_call catches bailout, function returns, file is dropped
    let result = callback.try_call(vec![]);

    if result.is_err() {
        // Bailout occurred, but file will still be closed
        // when this function returns
    }
}
```

## Solution 2: Using `BailoutGuard`

For cases where you need guaranteed cleanup even if bailout occurs directly
(not through `try_call`), use `BailoutGuard`:

```rust,ignore
use ext_php_rs::prelude::*;
use std::fs::File;

#[php_function]
pub fn process_file(callback: ZendCallable) {
    // Wrap the file handle in BailoutGuard
    let file = BailoutGuard::new(File::open("data.txt").unwrap());

    // Even if bailout occurs, the file will be closed
    callback.try_call(vec![]);

    // Use the file via Deref
    // file.read_to_string(...);
}
```

### How `BailoutGuard` Works

1. **Heap allocation**: The wrapped value is heap-allocated so it survives
   the `longjmp` stack unwinding.

2. **Cleanup registration**: A cleanup callback is registered in thread-local
   storage when the guard is created.

3. **On normal drop**: The cleanup is cancelled and the value is dropped normally.

4. **On bailout**: Before re-triggering the bailout, all registered cleanup
   callbacks are executed, dropping the guarded values.

### API

```rust,ignore
// Create a guard
let guard = BailoutGuard::new(value);

// Access the value (implements Deref and DerefMut)
guard.do_something();
let inner: &T = &*guard;
let inner_mut: &mut T = &mut *guard;

// Explicitly get references
let inner: &T = guard.get();
let inner_mut: &mut T = guard.get_mut();

// Extract the value, cancelling cleanup
let value: T = guard.into_inner();
```

### Performance Note

`BailoutGuard` incurs a heap allocation. Only use it for values that absolutely
must be cleaned up, such as:

- File handles
- Network connections
- Database connections
- Locks and mutexes
- Other system resources

For simple values without cleanup requirements, the overhead isn't worth it.

## Nested Calls

`BailoutGuard` works correctly with nested function calls. Guards at all
nesting levels are cleaned up when bailout occurs:

```rust,ignore
#[php_function]
pub fn outer_function(callback: ZendCallable) {
    let _outer_resource = BailoutGuard::new(Resource::new());

    inner_function(&callback);
}

fn inner_function(callback: &ZendCallable) {
    let _inner_resource = BailoutGuard::new(Resource::new());

    // If bailout occurs here, both inner and outer resources are cleaned up
    callback.try_call(vec![]);
}
```

## Best Practices

1. **Prefer `try_call`**: For most cases, using `try_call` and handling the
   error result is simpler and doesn't require heap allocation.

2. **Use `BailoutGuard` for critical resources**: Only wrap values that
   absolutely must be cleaned up (connections, locks, etc.).

3. **Don't overuse**: Not every value needs to be wrapped. Simple data
   structures without cleanup requirements don't need `BailoutGuard`.

4. **Combine approaches**: Use `try_call` where possible and `BailoutGuard`
   for critical resources that must be cleaned up regardless.
