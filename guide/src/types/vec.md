# `Vec`

Vectors can contain any type that can be represented as a zval. Note that the
data contained in the array will be copied into Rust types and stored inside the
vector. The internal representation of a PHP array is discussed below.

| `T` parameter | `&T` parameter | `T` Return type | `&T` Return type | PHP representation |
| ------------- | -------------- | --------------- | ---------------- | ------------------ |
| Yes           | No             | Yes             | No               | `ZendHashTable`    |

Internally, PHP arrays are hash tables where the key can be an unsigned long or
a string. Zvals are contained inside arrays therefore the data does not have to
contain only one type.

When converting into a vector, all values are converted from zvals into the
given generic type. If any of the conversions fail, the whole conversion will
fail.

## Rust example

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
#[php_function]
pub fn test_vec(vec: Vec<String>) -> String {
    vec.join(" ")
}
# fn main() {}
```

## PHP example

```php
<?php

var_dump(test_vec(['hello', 'world', 5])); // string(13) "hello world 5"
```

# `HashSet`

`HashSet` is an unordered collection of unique values. When converting to a PHP array,
values are stored with sequential integer keys (0, 1, 2, ...).

| `T` parameter | `&T` parameter | `T` Return type | `&T` Return type | PHP representation |
|---------------|----------------|-----------------|------------------|--------------------|
| Yes           | No             | Yes             | No               | `ZendHashTable`    |

## Rust example

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
use std::collections::HashSet;

#[php_function]
pub fn test_hashset(set: HashSet<String>) -> HashSet<String> {
    // Duplicates are automatically removed
    set
}
# fn main() {}
```

## PHP example

```php
<?php

// Duplicates are removed, order is not preserved
$result = test_hashset(['a', 'b', 'a', 'c']);
var_dump($result); // array with 3 unique values
```

# `BTreeSet`

`BTreeSet` is a sorted collection of unique values. Elements are ordered by their `Ord`
implementation. When converting to a PHP array, values are stored with sequential integer
keys (0, 1, 2, ...) in sorted order.

| `T` parameter | `&T` parameter | `T` Return type | `&T` Return type | PHP representation |
|---------------|----------------|-----------------|------------------|--------------------|
| Yes           | No             | Yes             | No               | `ZendHashTable`    |

## Rust example

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
use std::collections::BTreeSet;

#[php_function]
pub fn test_btreeset(set: BTreeSet<String>) -> BTreeSet<String> {
    // Duplicates removed, values sorted
    set
}
# fn main() {}
```

## PHP example

```php
<?php

// Values are sorted and deduplicated
$result = test_btreeset(['z', 'a', 'm', 'a']);
foreach ($result as $value) {
    echo "$value\n";
}
// a
// m
// z
```

# `IndexSet`

`IndexSet` is like `HashSet` but preserves insertion order. This requires the `indexmap`
feature.

```toml
[dependencies]
ext-php-rs = { version = "...", features = ["indexmap"] }
```

| `T` parameter | `&T` parameter | `T` Return type | `&T` Return type | PHP representation |
|---------------|----------------|-----------------|------------------|--------------------|
| Yes           | No             | Yes             | No               | `ZendHashTable`    |

## Rust example

```rust,ignore
use ext_php_rs::prelude::*;
use indexmap::IndexSet;

#[php_function]
pub fn test_indexset(set: IndexSet<String>) -> IndexSet<String> {
    // Order is preserved, duplicates removed
    for v in set.iter() {
        println!("v: {}", v);
    }
    set
}
```

## PHP example

```php
<?php

// Order is preserved, duplicates removed
$result = test_indexset(['z', 'a', 'm', 'a']);
foreach ($result as $value) {
    echo "$value\n";
}
// z
// a
// m
```
