# Functions & methods

PHP functions and methods are represented by the `Function` struct.

You can use the `try_from_function` and `try_from_method` methods to obtain a Function struct corresponding to the passed function or static method name.
It's heavily recommended you reuse returned `Function` objects, to avoid the overhead of looking up the function/method name.

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

## Cached Callables

When calling the same PHP function repeatedly from Rust, use `CachedCallable`
to avoid re-resolving the function on every call. The first resolution caches
the internal `zend_fcall_info_cache`, and subsequent calls skip all string
lookups and hash table searches.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::ZendCallable;

#[php_function]
pub fn call_many_times(callback: ZendCallable) -> () {
    let cached = callback.cache().expect("Failed to cache callable");

    for i in 0..1000i64 {
        let _ = cached.try_call(vec![&i]);
    }
}
# fn main() {}
```

### When to use `CachedCallable`

- **Use `CachedCallable`** when calling the same callable multiple times (loops,
  event handlers, iterators like `array_map` patterns).
- **Use `ZendCallable`** for single-shot calls where caching overhead is wasted.

### Error handling

`CachedCallable` returns `CachedCallableError` which provides granular
error variants:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::error::CachedCallableError;
use ext_php_rs::types::ZendCallable;

#[php_function]
pub fn resilient_caller(callback: ZendCallable) -> () {
    let cached = callback.cache().expect("Failed to cache");

    match cached.try_call(vec![&42i64]) {
        Ok(result) => { /* use result */ },
        Err(CachedCallableError::PhpException(_)) => {
            // PHP exception — callable is still valid, can retry
            let _ = cached.try_call(vec![&0i64]);
        },
        Err(CachedCallableError::Poisoned) => {
            // Engine failure happened before — cannot reuse
        },
        Err(e) => { /* other errors */ },
    }
}
# fn main() {}
```

### Named arguments

`CachedCallable` supports the same named argument methods as `ZendCallable`:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::ZendCallable;

#[php_function]
pub fn cached_with_named() -> () {
    let func = ZendCallable::try_from_name("str_replace").unwrap();
    let cached = func.cache().unwrap();

    let _ = cached.try_call_named(&[
        ("search", &"world"),
        ("replace", &"PHP"),
        ("subject", &"Hello world"),
    ]);
}
# fn main() {}
```
