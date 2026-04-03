# Module Globals

PHP extensions can declare per-module global state that is automatically managed
by the engine. In ZTS (thread-safe) builds, PHP's TSRM allocates a separate copy
of the globals for each thread. In non-ZTS builds, the globals are a plain
static variable.

ext-php-rs exposes this via `ModuleGlobals<T>` and the `ModuleGlobal` trait.

## Defining Globals

Create a struct that implements `Default` and `ModuleGlobal`:

```rust,ignore
use ext_php_rs::zend::{ModuleGlobal, ModuleGlobals};

#[derive(Default)]
struct MyGlobals {
    request_count: i64,
    max_depth: i32,
}

impl ModuleGlobal for MyGlobals {
    fn ginit(&mut self) {
        self.max_depth = 512;
    }
}
```

`Default::default()` initializes the struct, then `ginit()` runs for any
additional setup. In ZTS mode these callbacks fire once per thread; in non-ZTS
mode they fire once at module load.

If you don't need custom initialization, leave the trait impl empty:

```rust,ignore
# use ext_php_rs::zend::{ModuleGlobal, ModuleGlobals};
#[derive(Default)]
struct SimpleGlobals {
    counter: i64,
}

impl ModuleGlobal for SimpleGlobals {}
```

## Registering Globals

Declare a `static` and pass it to `ModuleBuilder::globals()`:

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::zend::{ModuleGlobal, ModuleGlobals};

# #[derive(Default)]
# struct MyGlobals { request_count: i64, max_depth: i32 }
# impl ModuleGlobal for MyGlobals {}
#
static MY_GLOBALS: ModuleGlobals<MyGlobals> = ModuleGlobals::new();

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module.globals(&MY_GLOBALS)
}
```

Only one globals struct per module is supported (a PHP limitation).

## Accessing Globals

Use `get()` for shared access and `get_mut()` for mutable access:

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::zend::{ModuleGlobal, ModuleGlobals};

# #[derive(Default)]
# struct MyGlobals { request_count: i64, max_depth: i32 }
# impl ModuleGlobal for MyGlobals {}
# static MY_GLOBALS: ModuleGlobals<MyGlobals> = ModuleGlobals::new();
#
#[php_function]
pub fn get_request_count() -> i64 {
    MY_GLOBALS.get().request_count
}

#[php_function]
pub fn increment_request_count() {
    unsafe { MY_GLOBALS.get_mut() }.request_count += 1;
}
```

`get()` is safe because PHP runs one request per thread at a time. `get_mut()`
is `unsafe` because the caller must ensure exclusive access (which is guaranteed
within a single `#[php_function]` handler, but not from background Rust threads).

## Advanced: Raw Pointer Access

For power users who need direct pointer access (e.g., passing to C APIs or
building custom lock-free patterns), `as_ptr()` returns `*mut T`:

```rust,ignore
# use ext_php_rs::zend::{ModuleGlobal, ModuleGlobals};
# #[derive(Default)]
# struct MyGlobals { request_count: i64 }
# impl ModuleGlobal for MyGlobals {}
# static MY_GLOBALS: ModuleGlobals<MyGlobals> = ModuleGlobals::new();
let ptr: *mut MyGlobals = MY_GLOBALS.as_ptr();
```

## Cleanup

Implement `gshutdown()` if your globals hold external resources:

```rust,ignore
# use ext_php_rs::zend::ModuleGlobal;
# #[derive(Default)]
# struct MyGlobals { handle: Option<u64> }
impl ModuleGlobal for MyGlobals {
    fn gshutdown(&mut self) {
        self.handle.take();
    }
}
```

The struct is also dropped after `gshutdown()` returns, so standard `Drop`
implementations work as expected.
