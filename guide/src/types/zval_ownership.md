# Zval Ownership: `Separated` & `PhpRef`

In PHP, there is a distinction between passing a value normally (`$x`) and
passing it by reference (`&$x`). In ext-php-rs, a parameter type declares
pass-by-reference through `FromZvalMut::BY_REF`. `&mut Zval`, `PhpRef` and
`&mut ZendHashTable` set PHP's `ZEND_SEND_BY_REF` flag. The flag forces callers
to pass a variable, so a literal like `foo([1, 2, 3])` is rejected at runtime.

Object parameters (`&mut ZendObject`, `&mut MyClass`) are passed by value. PHP
objects are handles, so the caller sees every mutation without the flag.

`Separated` and `PhpRef` decouple Rust mutability from PHP's pass-by-reference
semantics.

## When to use which

| Type | PHP syntax | Modifies caller? | Use case |
|---|---|---|---|
| `Separated` | `foo($x)` or `foo([1,2])` | No | Mutate a local copy (COW) |
| `PhpRef` | `foo(&$x)` | Yes | Modify the caller's variable |
| `&Zval` | `foo($x)` | No | Read-only access |
| `&mut Zval` | `foo(&$x)` | Yes | Legacy, prefer `PhpRef` |
| `&mut ZendHashTable` | `foo(&$x)` | Yes | Mutate the caller's array in place |
| `&mut MyClass` | `foo($x)` or `foo(new MyClass)` | Yes | Objects are handles |

## `Separated` — local mutation without pass-by-reference

`Separated` wraps `&mut Zval` but does **not** set `ZEND_SEND_BY_REF`. PHP
callers can pass any value, including literals. Call `.array_mut()` to trigger
Copy-on-Write separation before mutating arrays.

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::types::Separated;

#[php_function]
pub fn process_array(mut data: Separated) -> bool {
    let Some(ht) = data.array_mut() else {
        return false;
    };
    ht.push("appended").is_ok()
}
```

```php
// Both work — no & required:
process_array([1, 2, 3]);

$arr = [1, 2, 3];
process_array($arr);
// $arr is unchanged — COW separation protects it
```

Since `Separated` implements `Deref<Target = Zval>`, all read methods
(`long()`, `str()`, `array()`, `object()`, etc.) are available directly.

## `PhpRef` — modify the caller's variable

`PhpRef` is the explicit opt-in for PHP pass-by-reference. It sets
`ZEND_SEND_BY_REF`, so the caller **must** pass a variable (not a literal).
Mutations affect the original variable.

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::types::PhpRef;

#[php_function]
pub fn increment(mut val: PhpRef) {
    if let Some(n) = val.long() {
        val.set_long(n + 1);
    }
}
```

```php
$x = 5;
increment($x);
// $x is now 6

// increment(42); // Error: cannot pass by reference
```

## How it works

Both types are `#[repr(transparent)]` newtypes over `&mut Zval` with zero
runtime overhead. The difference is one constant on their `FromZvalMut`
implementation: `PhpRef` sets `BY_REF = true`, `Separated` keeps the default
`false`. The macro emits `Arg::of::<T>(name)`, which reads that constant, so a
type alias of `PhpRef` is also passed by reference.

The `Zval::array_mut()` method already implements PHP's `SEPARATE_ARRAY()`
semantics — it duplicates the underlying hashtable when the refcount is greater
than 1. This is what makes `Separated` safe: the caller's original value is
never modified.
