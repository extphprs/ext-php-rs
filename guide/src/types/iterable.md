# `Iterable`

`Iterable`s are represented either by an `array` or `Traversable` type.

| `T` parameter | `&T` parameter | `T` Return type | `&T` Return type | PHP representation               |
|---------------|----------------|-----------------| ---------------- |----------------------------------|
| Yes           | No             | No              | No               | `ZendHashTable` or `ZendIterator` |

Converting from a zval to a `Iterable` is valid when the value is either an array or an object
that implements the `Traversable` interface. This means that any value that can be used in a
`foreach` loop can be converted into a `Iterable`.

Both variants yield owned `(Zval, Zval)` pairs; see [`ZendIterator`](./iterator.md)
for why the `Traversable` side cannot lend references.

## Rust example

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
# use ext_php_rs::types::Iterable;
#[php_function]
pub fn test_iterable(mut iterable: Iterable) {
    for (k, v) in iterable.iter().expect("cannot rewind iterator") {
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

$array = [
    'hello' => 'world',
    'rust' => 'php',
];

test_iterable($generator());
test_iterable($array);
```

Output:

```text
k: hello v: world
k: rust v: php
k: hello v: world
k: rust v: php
```
