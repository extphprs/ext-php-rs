# `HashMap`

`HashMap`s are represented as associative arrays in PHP.

| `T` parameter | `&T` parameter | `T` Return type | `&T` Return type | PHP representation |
|---------------|----------------|-----------------|------------------|--------------------|
| Yes           | No             | Yes             | No               | `ZendHashTable`    |

Converting from a zval to a `HashMap` is valid when the key is a `String`, and
the value implements `FromZval`. The key and values are copied into Rust types
before being inserted into the `HashMap`. If one of the key-value pairs has a
numeric key, the key is represented as a string before being inserted.

Converting from a `HashMap` to a zval is valid when the key implements
`AsRef<str>`, and the value implements `IntoZval`.

<div class="warning">

    When using `HashMap` the order of the elements is not preserved.

    HashMaps are unordered collections, so the order of elements may not be the same
    when converting from PHP to Rust and back.

    If you need to preserve the order of elements, consider using `IndexMap` (requires
    the `indexmap` feature) or `Vec<(K, V)>`.
</div>

## Rust example

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
# use std::collections::HashMap;
#[php_function]
pub fn test_hashmap(hm: HashMap<String, String>) -> Vec<String> {
    for (k, v) in hm.iter() {
        println!("k: {} v: {}", k, v);
    }

    hm.into_iter()
        .map(|(_, v)| v)
        .collect::<Vec<_>>()
}
# fn main() {}
```

## PHP example

```php
<?php

var_dump(test_hashmap([
    'hello' => 'world',
    'rust' => 'php',
    'okk',
]));
```

Output:

```text
k: rust v: php
k: hello v: world
k: 0 v: okk
array(3) {
    [0] => string(3) "php",
    [1] => string(5) "world",
    [2] => string(3) "okk"
}
```

## `IndexMap` (Order-Preserving)

`IndexMap` is like `HashMap` but preserves insertion order, making it ideal for
working with PHP arrays where key order matters. This requires the `indexmap`
feature.

```toml
[dependencies]
ext-php-rs = { version = "...", features = ["indexmap"] }
```

| `T` parameter | `&T` parameter | `T` Return type | `&T` Return type | PHP representation |
|---------------|----------------|-----------------|------------------|--------------------|
| Yes           | No             | Yes             | No               | `ZendHashTable`    |

### Rust example

```rust,ignore
use ext_php_rs::prelude::*;
use indexmap::IndexMap;

#[php_function]
pub fn test_indexmap(map: IndexMap<String, String>) -> IndexMap<String, i32> {
    // Order is preserved when iterating
    for (k, v) in map.iter() {
        println!("k: {} v: {}", k, v);
    }

    // Return an ordered map
    let mut result = IndexMap::new();
    result.insert("first".to_string(), 1);
    result.insert("second".to_string(), 2);
    result.insert("third".to_string(), 3);
    result  // Order preserved: first, second, third
}
```

### PHP example

```php
<?php

// Keys maintain their order
$result = test_indexmap([
    'z' => 'last',
    'a' => 'first',
    'm' => 'middle',
]);

// Output preserves insertion order
foreach ($result as $key => $value) {
    echo "$key => $value\n";
}
// first => 1
// second => 2
// third => 3
```

## `Vec<(K, V)>` and `Vec<ArrayKey, V>`

`Vec<(K, V)>` and `Vec<ArrayKey, V>` are used to represent associative arrays in PHP
where the keys can be strings or integers.

If using `String` or `&str` as the key type, only string keys will be accepted.

For `i64` keys, string keys that can be parsed as integers will be accepted, and
converted to `i64`.

If you need to accept both string and integer keys, use `ArrayKey` as the key type.

### Rust example

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
# use ext_php_rs::types::ArrayKey;
#[php_function]
pub fn test_vec_kv(vec: Vec<(String, String)>) -> Vec<String> {
    for (k, v) in vec.iter() {
        println!("k: {} v: {}", k, v);
    }

    vec.into_iter()
        .map(|(_, v)| v)
        .collect::<Vec<_>>()
}

#[php_function]
pub fn test_vec_arraykey(vec: Vec<(ArrayKey, String)>) -> Vec<String> {
    for (k, v) in vec.iter() {
        println!("k: {} v: {}", k, v);
    }

    vec.into_iter()
        .map(|(_, v)| v)
        .collect::<Vec<_>>()
}
# fn main() {}
```

## PHP example

```php
<?php

declare(strict_types=1);

var_dump(test_vec_kv([
    ['hello', 'world'],
    ['rust', 'php'],
    ['okk', 'okk'],
]));

var_dump(test_vec_arraykey([
    ['hello', 'world'],
    [1, 'php'],
    ["2", 'okk'],
]));
```

Output:

```text
k: hello v: world
k: rust v: php
k: okk v: okk
array(3) {
    [0] => string(5) "world",
    [1] => string(3) "php",
    [2] => string(3) "okk"
}
k: hello v: world
k: 1 v: php
k: 2 v: okk
array(3) {
    [0] => string(5) "world",
    [1] => string(3) "php",
    [2] => string(3) "okk"
}
```
