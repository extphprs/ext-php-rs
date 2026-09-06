# `ZendIterator`

`ZendIterator`s are represented by the `Traversable` type.

| `T` parameter | `&T` parameter | `T` Return type | `&T` Return type | PHP representation |
|---------------| -------------- |-----------------| ---------------- | ------------------ |
| No            | Yes            | No              | No               | `ZendIterator`    |

Converting from a zval to a `ZendIterator` is valid when there is an associated iterator to
the variable. This means that any value, at the exception of an `array`, that can be used in
a `foreach` loop can be converted into a `ZendIterator`. As an example, a `Generator` can be
used but also a the result of a `query` call with `PDO`.

If you want a more universal `iterable` type that also supports arrays, see [Iterable](./iterable.md).

Each iteration yields an owned `(Zval, Zval)` pair. The value is a shallow clone
of the zval Zend hands out, because the engine releases its own copy on the
next `move_forward`; this is the same copy `foreach` performs.

## Rust example

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
# use ext_php_rs::types::ZendIterator;
#[php_function]
pub fn test_iterator(iterator: &mut ZendIterator) {
    for (k, v) in iterator.iter().expect("cannot rewind iterator") {
        // Note that the key can be anything, even an object
        // when iterating over Traversables!
        println!("k: {} v: {}", k.string().unwrap(), v.string().unwrap());
    }
}
# fn main() {}
```

## PHP example

```php
<?php

$generator = function() {
    yield 'hello' => 'world';
    yield 'rust' => 'php';
};

test_iterator($generator());
```

Output:

```text
k: hello v: world
k: rust v: php
```
