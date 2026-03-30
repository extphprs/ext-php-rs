# Zval Ownership: `Separated` & `PhpRef`

In PHP, there is a distinction between passing a value normally (`$x`) and
passing it by reference (`&$x`). In ext-php-rs, using `&mut Zval` in a function
signature sets PHP's `ZEND_SEND_BY_REF` flag, which forces callers to pass by
reference — meaning literals like `foo([1, 2, 3])` are rejected at runtime.

`Separated` and `PhpRef` fix this by decoupling Rust mutability from PHP's
pass-by-reference semantics.

## When to use which

| Type | PHP syntax | Modifies caller? | Use case |
|---|---|---|---|
| `Separated` | `foo($x)` or `foo([1,2])` | No | Mutate a local copy (COW) |
| `PhpRef` | `foo(&$x)` | Yes | Modify the caller's variable |
| `&Zval` | `foo($x)` | No | Read-only access |
| `&mut Zval` | `foo(&$x)` | Yes | Legacy — prefer `PhpRef` |

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
runtime overhead. The difference is purely in the proc macro:

- `Separated` → macro emits `Arg::new(...)` (no pass-by-ref flag)
- `PhpRef` → macro emits `Arg::new(...).as_ref()` (sets pass-by-ref flag)

The `Zval::array_mut()` method already implements PHP's `SEPARATE_ARRAY()`
semantics — it duplicates the underlying hashtable when the refcount is greater
than 1. This is what makes `Separated` safe: the caller's original value is
never modified.
