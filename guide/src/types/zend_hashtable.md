# `ZendHashTable`

`ZendHashTable` is the internal representation of PHP arrays. While you can use
`Vec` and `HashMap` for most use cases (which are converted to/from
`ZendHashTable` automatically), working directly with `ZendHashTable` gives you
more control and avoids copying data when you need to manipulate PHP arrays
in-place.

## When to use `ZendHashTable` directly

- When you need to modify a PHP array in place without copying
- When working with arrays passed by reference
- When you need fine-grained control over array operations
- When implementing custom iterators or data structures

## Basic Operations

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::ZendHashTable;
use ext_php_rs::boxed::ZBox;

#[php_function]
pub fn create_array() -> ZBox<ZendHashTable> {
    let mut ht = ZendHashTable::new();

    // Push values (auto-incrementing numeric keys)
    ht.push("first").unwrap();
    ht.push("second").unwrap();

    // Insert with string keys
    ht.insert("name", "John").unwrap();
    ht.insert("age", 30i64).unwrap();

    // Insert at specific numeric index
    ht.insert_at_index(100, "at index 100").unwrap();

    ht
}

#[php_function]
pub fn read_array(arr: &ZendHashTable) {
    // Get by string key
    if let Some(name) = arr.get("name") {
        println!("Name: {:?}", name.str());
    }

    // Get by numeric index
    if let Some(first) = arr.get_index(0) {
        println!("First: {:?}", first.str());
    }

    // Check length
    println!("Length: {}", arr.len());
    println!("Is empty: {}", arr.is_empty());

    // Iterate over key-value pairs
    for (key, value) in arr.iter() {
        println!("{}: {:?}", key, value);
    }
}
# fn main() {}
```

## Entry API

The Entry API provides an ergonomic way to handle hash table operations where
you need to conditionally insert or update values based on whether a key already
exists. This is similar to Rust's `std::collections::hash_map::Entry` API.

### Basic Usage

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::ZendHashTable;
use ext_php_rs::boxed::ZBox;

#[php_function]
pub fn entry_example() -> ZBox<ZendHashTable> {
    let mut ht = ZendHashTable::new();

    // Insert a default value if the key doesn't exist
    ht.entry("counter").or_insert(0i64).unwrap();

    // Modify the value if it exists, using and_modify
    ht.entry("counter")
        .and_modify(|v| {
            if let Some(n) = v.long() {
                v.set_long(n + 1);
            }
        })
        .or_insert(0i64)
        .unwrap();

    // Use or_insert_with for lazy initialization
    ht.entry("computed")
        .or_insert_with(|| "computed value")
        .unwrap();

    // Works with numeric keys too
    ht.entry(42i64).or_insert("value at index 42").unwrap();

    ht
}
# fn main() {}
```

### Entry Variants

The `entry()` method returns an `Entry` enum with two variants:

- `Entry::Occupied` - The key exists in the hash table
- `Entry::Vacant` - The key does not exist

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{ZendHashTable, Entry, Zval};
use ext_php_rs::boxed::ZBox;

#[php_function]
pub fn match_entry() -> ZBox<ZendHashTable> {
    let mut ht = ZendHashTable::new();
    ht.insert("existing", "value").unwrap();

    // Pattern match on the entry
    match ht.entry("existing") {
        Entry::Occupied(entry) => {
            println!("Key {:?} exists with value {:?}",
                     entry.key(),
                     entry.get().and_then(Zval::str),
             );
        }
        Entry::Vacant(entry) => {
            println!("Key {:?} is vacant", entry.key());
            entry.insert("new value").unwrap();
        }
    }

    ht
}
# fn main() {}
```

### Common Patterns

#### Counting occurrences

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::ZendHashTable;
use ext_php_rs::boxed::ZBox;

#[php_function]
pub fn count_words(words: Vec<String>) -> ZBox<ZendHashTable> {
    let mut counts = ZendHashTable::new();

    for word in words {
        counts.entry(word.as_str())
            .and_modify(|v| {
                if let Some(n) = v.long() {
                    v.set_long(n + 1);
                }
            })
            .or_insert(1i64)
            .unwrap();
    }

    counts
}
# fn main() {}
```

#### Caching computed values

This example demonstrates using `or_insert_with_key` for lazy computation:

```rust,no_run
# extern crate ext_php_rs;
use ext_php_rs::types::ZendHashTable;

fn expensive_computation(key: &str) -> String {
    format!("computed_{}", key)
}

fn get_or_compute(cache: &mut ZendHashTable, key: &str) -> String {
    let value = cache.entry(key)
        .or_insert_with_key(|k| expensive_computation(&k.to_string()))
        .unwrap();

    value.str().unwrap_or_default().to_string()
}
# fn main() {}
```

#### Updating existing values

This example shows how to conditionally update a value only if the key exists:

```rust,no_run
# extern crate ext_php_rs;
use ext_php_rs::types::{ZendHashTable, Entry};

fn update_if_exists(ht: &mut ZendHashTable, key: &str, new_value: &str) -> bool {
    match ht.entry(key) {
        Entry::Occupied(mut entry) => {
            entry.insert(new_value).unwrap();
            true
        }
        Entry::Vacant(_) => false,
    }
}
# fn main() {}
```

### Entry Methods Reference

#### `Entry` methods

| Method                  | Description                                    |
|-------------------------|------------------------------------------------|
| `or_insert(default)`    | Insert `default` if vacant, return `&mut Zval` |
| `or_insert_with(f)`     | Insert result of `f()` if vacant               |
| `or_insert_with_key(f)` | Insert result of `f(&key)` if vacant           |
| `or_default()`          | Insert default `Zval` (null) if vacant         |
| `key()`                 | Get reference to the key                       |
| `and_modify(f)`         | Modify value in place if occupied              |

#### `OccupiedEntry` methods

| Method           | Description                                        |
|------------------|----------------------------------------------------|
| `key()`          | Get reference to key                               |
| `get()`          | Get reference to value                             |
| `get_mut()`      | Get mutable reference to value                     |
| `into_mut()`     | Convert to mutable reference with entry's lifetime |
| `insert(value)`  | Replace value, returning old value                 |
| `remove()`       | Remove and return value                            |
| `remove_entry()` | Remove and return key-value pair                   |

#### `VacantEntry` methods

| Method          | Description                         |
|-----------------|-------------------------------------|
| `key()`         | Get reference to key                |
| `into_key()`    | Take ownership of key               |
| `insert(value)` | Insert value and return `&mut Zval` |

## PHP Example

```php
<?php

// Using the create_array function
$arr = create_array();
var_dump($arr);
// array(5) {
//   [0]=> string(5) "first"
//   [1]=> string(6) "second"
//   ["name"]=> string(4) "John"
//   ["age"]=> int(30)
//   [100]=> string(12) "at index 100"
// }

// Count words
$counts = count_words(['apple', 'banana', 'apple', 'cherry', 'banana', 'apple']);
var_dump($counts);
// array(3) {
//   ["apple"]=> int(3)
//   ["banana"]=> int(2)
//   ["cherry"]=> int(1)
// }
```
