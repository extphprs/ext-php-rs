# Bailout Guard

A bailout is a jump that the engine makes with `longjmp` when it cannot continue.
A fatal error, `memory_limit` exhaustion, or `E_USER_ERROR` cause a bailout. The
jump skips the Rust frames between the error and the catch point. Rust does not
run the destructors of the values in those frames. File handles, connections, and
locks leak.

On Windows, the MSVC `longjmp` unwinds the Rust frames and their destructors
run. Do not rely on this. Linux and macOS skip them.

`exit()` and `die()` do not cause a bailout on PHP 8. They throw an internal
exception. The handler returns normally and Rust runs the destructors.

## The problem

```rust,ignore
#[php_function]
pub fn process_file(callback: ZendCallable) {
    let file = File::open("data.txt").unwrap();

    // A fatal error in the callback leaks the file handle
    callback.try_call(vec![]);
}
```

`try_call` does not catch a bailout. The jump crosses `process_file` and stops
at the `try_catch` that wraps every exported function. Rust never drops `file`.

## Using `BailoutGuard`

```rust,ignore
use ext_php_rs::prelude::*;
use std::fs::File;

#[php_function]
pub fn process_file(callback: ZendCallable) {
    let file = BailoutGuard::new(File::open("data.txt").unwrap());

    // The file is closed even if a bailout occurs
    callback.try_call(vec![]);

    // Use the file via Deref
    // file.read_to_string(...);
}
```

### How `BailoutGuard` works

A `try_catch` frame is a call to `ext_php_rs::zend::try_catch`. Every exported
PHP function runs inside one. `Embed::run` and `Embed::eval` open one too.

1. `BailoutGuard::new` moves the value to the heap. The value survives the
   `longjmp`.
2. `BailoutGuard::new` registers a cleanup entry with the innermost `try_catch`
   frame.
3. When you drop the guard, the guard releases the entry and drops the value.
4. When a bailout occurs, the `try_catch` frame that catches it drops the guards
   that were created inside its closure, newest first. Then it returns
   `Err(CatchError::Bailout)`. The guards that were created before that frame
   are not changed. You can still use them.

`BailoutGuard` is `!Send`. The guard belongs to the thread that created it.

Do not move a guard out of a `try_catch` closure through shared mutable state.
If that closure bails out, the frame drops the value and the moved guard points
at freed memory.

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

// Extract the value and release the cleanup entry
let value: T = guard.into_inner();
```

### Performance

`BailoutGuard::new` makes one heap allocation. Use it only for values that must
be released:

- File handles
- Network connections
- Database connections
- Locks and mutexes
- Other system resources

Do not wrap simple values. The allocation costs more than the leak.

## Nested calls

The catching `try_catch` drops the guards of every Rust call level between the
bailout and itself:

```rust,ignore
#[php_function]
pub fn outer_function(callback: ZendCallable) {
    let _outer_resource = BailoutGuard::new(Resource::new());

    inner_function(&callback);
}

fn inner_function(callback: &ZendCallable) {
    let _inner_resource = BailoutGuard::new(Resource::new());

    // On bailout, the try_catch of outer_function drops both resources
    callback.try_call(vec![]);
}
```

PHP code that `outer_function` calls can call a second exported Rust function.
If that function bails out, its own `try_catch` drops only its own guards. Then
it triggers the bailout again. The frame of `outer_function` drops
`_outer_resource`.

## Catching a bailout yourself

If you must continue after a failed engine call, open your own `try_catch`:

```rust,ignore
use ext_php_rs::zend::try_catch;

let connection = BailoutGuard::new(Connection::open());

let result = try_catch(|| {
    let _tmp = BailoutGuard::new(TempFile::create());
    risky_engine_call();
});

// On Err, try_catch dropped _tmp. connection is still valid here.
connection.query("...");
```
