# Functions & methods

PHP functions and methods can be called from Rust using several approaches,
depending on your performance needs.

## Function Struct

The `Function` struct represents a PHP function or method. You can use
`try_from_function` and `try_from_method` to obtain a `Function` struct
corresponding to the passed function or static method name.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::Function;

#[php_function]
pub fn test_function() -> () {
    let var_dump = Function::try_from_function("var_dump").unwrap();
    let _ = var_dump.try_call(vec![&"abc"]);
}

#[php_function]
pub fn test_method() -> () {
    let f = Function::try_from_method("ClassName", "staticMethod").unwrap();
    let _ = f.try_call(vec![&"abc"]);
}

# fn main() {}
```

## ZendCallable

`ZendCallable` wraps a PHP callable value (function name, closure, or
`[$object, 'method']` array) and allows you to call it from Rust. It uses lazy
caching - the function lookup is performed on the first call and cached for
subsequent calls.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::ZendCallable;

#[php_function]
pub fn call_callback(callback: ZendCallable) -> String {
    let result = callback.try_call(vec![&"hello"]).unwrap();
    result.string().unwrap().to_string()
}

# fn main() {}
```

## CachedCallable

`CachedCallable` provides eager caching - the function lookup is performed at
construction time. This allows early validation that the callable exists,
failing fast if the function doesn't exist rather than discovering the error
on first call.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::CachedCallable;

#[php_function]
pub fn process_items(items: Vec<String>) -> Vec<String> {
    // Cache the function lookup once
    let mut strtoupper = CachedCallable::try_from_name("strtoupper").unwrap();

    items
        .iter()
        .map(|item| {
            // Repeated calls use the cached function pointer
            strtoupper
                .try_call(vec![item])
                .unwrap()
                .string()
                .unwrap()
                .to_string()
        })
        .collect()
}

# fn main() {}
```

### Creating a CachedCallable

There are several ways to create a `CachedCallable`:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::types::{CachedCallable, Zval};

# fn example() -> ext_php_rs::error::Result<()> {
// From a function name
let mut callable = CachedCallable::try_from_name("strtoupper")?;

// From a &str (same as try_from_name)
let mut callable = CachedCallable::try_from("array_map")?;

// From a Zval containing a callable
let zval = Zval::new();
// ... set zval to a callable value ...
// let mut callable = CachedCallable::try_from_zval(zval)?;
# Ok(())
# }
# fn main() {}
```

### Calling Methods

You can also call the function with pre-converted `Zval` arguments using
`call_with_zvals` for even more control:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{CachedCallable, Zval};
use ext_php_rs::convert::IntoZval;

#[php_function]
pub fn example() -> i64 {
    let mut callable = CachedCallable::try_from_name("max").unwrap();

    // Using try_call with auto-conversion
    let result = callable.try_call(vec![&1i64, &5i64, &3i64]).unwrap();

    // Or using call_with_zvals with pre-converted values
    let args: Vec<Zval> = vec![
        1i64.into_zval(false).unwrap(),
        5i64.into_zval(false).unwrap(),
        3i64.into_zval(false).unwrap(),
    ];
    let result = callable.call_with_zvals(&args).unwrap();

    result.long().unwrap()
}

# fn main() {}
```

## Performance Considerations

Both `ZendCallable` and `CachedCallable` cache function lookups for efficient
repeated calls. The difference is *when* the caching happens:

- **`Function`**: Performs a function lookup on each `try_from_*` call. Good for
  one-off calls.
- **`ZendCallable`**: Lazy caching - validates and caches on first `try_call`.
  Use when receiving callbacks from PHP or when you may not call the function.
- **`CachedCallable`**: Eager caching - validates and caches at construction.
  Use when you want to fail fast if the function doesn't exist.
