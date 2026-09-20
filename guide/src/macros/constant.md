# `#[php_const]` Attribute

Exports a Rust constant as a global PHP constant. The constant can be any type
that implements `IntoConst`.

The `wrap_constant!()` macro can be used to simplify the registration of constants.
It sets the name and doc comments for the constant.

You can rename the const with options:

- `name` - Allows you to rename the property, e.g.
  `#[php(name = "new_name")]`
- `change_case` - Allows you to rename the property using rename rules, e.g.
  `#[php(change_case = PascalCase)]`

A global constant has no visibility. `#[php(vis = "...")]` on a `#[php_const]`
is a compile error. Use `vis` on constants inside a `#[php_impl]` block.

## Examples

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

#[php_const]
const TEST_CONSTANT: i32 = 100;

#[php_const]
#[php(name = "I_AM_RENAMED")]
const TEST_CONSTANT_THE_SECOND: i32 = 42;

#[php_const]
const ANOTHER_STRING_CONST: &'static str = "Hello world!";

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .constant(wrap_constant!(TEST_CONSTANT))
        .constant(wrap_constant!(TEST_CONSTANT_THE_SECOND))
        .constant(("MANUAL_CONSTANT", ANOTHER_STRING_CONST, &[]))
}
# fn main() {}
```

## PHP usage

```php
<?php

var_dump(TEST_CONSTANT); // int(100)
var_dump(I_AM_RENAMED); // int(42)
var_dump(MANUAL_CONSTANT); // string(12) "Hello world!"
```
