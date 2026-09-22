# `#[php]` Attributes

There are a number of attributes that can be used to annotate elements in your
extension.

Multiple `#[php]` attributes will be combined. For example, the following will
be identical:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
#[php_function]
#[php(name = "hi_world")]
#[php(defaults(a = 1, b = 2))]
fn hello_world(a: i32, b: i32) -> i32 {
    a + b
}
# fn main() {}
```

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
#[php_function]
#[php(name = "hi_world", defaults(a = 1, b = 2))]
fn hello_world(a: i32, b: i32) -> i32 {
    a + b
}
# fn main() {}
```

Which attributes are available depends on the element you are annotating:

| Attribute                  | `const` | `fn` | `struct` | `struct` field | `impl` | `impl` `const` | `impl` `fn` | `enum` | `enum` case |
| -------------------------- | ------- | ---- | -------- | -------------- | ------ | -------------- | ----------- | ------ | ----------- |
| name                       | ✅      | ✅   | ✅       | ✅             | ❌     | ✅             | ✅          | ✅     | ✅          |
| change_case                | ✅      | ✅   | ✅       | ✅             | ❌     | ✅             | ✅          | ✅     | ✅          |
| change_method_case         | ❌      | ❌   | ❌       | ❌             | ✅     | ❌             | ❌          | ❌     | ❌          |
| change_constant_case       | ❌      | ❌   | ❌       | ❌             | ✅     | ❌             | ❌          | ❌     | ❌          |
| flags                      | ❌      | ❌   | ✅       | ✅             | ❌     | ❌             | ❌          | ❌     | ❌          |
| prop                       | ❌      | ❌   | ❌       | ✅             | ❌     | ❌             | ❌          | ❌     | ❌          |
| static                     | ❌      | ❌   | ❌       | ✅             | ❌     | ❌             | ❌          | ❌     | ❌          |
| default                    | ❌      | ❌   | ❌       | ✅ (static)    | ❌     | ❌             | ❌          | ❌     | ❌          |
| readonly                   | ❌      | ❌   | ✅       | ❌             | ❌     | ❌             | ❌          | ❌     | ❌          |
| extends                    | ❌      | ❌   | ✅       | ❌             | ❌     | ❌             | ❌          | ❌     | ❌          |
| implements                 | ❌      | ❌   | ✅       | ❌             | ❌     | ❌             | ❌          | ❌     | ❌          |
| modifier                   | ❌      | ❌   | ✅       | ❌             | ❌     | ❌             | ❌          | ❌     | ❌          |
| defaults                   | ❌      | ✅   | ❌       | ❌             | ❌     | ❌             | ✅          | ❌     | ❌          |
| optional                   | ❌      | ✅   | ❌       | ❌             | ❌     | ❌             | ✅          | ❌     | ❌          |
| vis                        | ❌      | ❌   | ❌       | ❌             | ❌     | ✅             | ✅          | ❌     | ❌          |
| getter                     | ❌      | ❌   | ❌       | ❌             | ❌     | ❌             | ✅          | ❌     | ❌          |
| setter                     | ❌      | ❌   | ❌       | ❌             | ❌     | ❌             | ✅          | ❌     | ❌          |
| constructor                | ❌      | ❌   | ❌       | ❌             | ❌     | ❌             | ✅          | ❌     | ❌          |
| abstract                   | ❌      | ❌   | ❌       | ❌             | ❌     | ❌             | ✅          | ❌     | ❌          |
| final                      | ❌      | ❌   | ❌       | ❌             | ❌     | ❌             | ✅          | ❌     | ❌          |
| allow_native_discriminants | ❌      | ❌   | ❌       | ❌             | ❌     | ❌             | ❌          | ✅     | ❌          |
| value                      | ❌      | ❌   | ❌       | ❌             | ❌     | ❌             | ❌          | ❌     | ✅          |
| rename_cases               | ❌      | ❌   | ❌       | ❌             | ❌     | ❌             | ❌          | ✅     | ❌          |

An option marked ❌ is a compile error. The error points at the option and
names the item where the option is valid. The proc macros themselves take no
arguments: write `#[php_class]` and `#[php(name = "Foo")]`, not
`#[php_class(name = "Foo")]`.

Trait methods inside `#[php_interface]` accept `name`, `change_case`,
`defaults`, `optional` and `vis`. Trait constants accept `name` and
`change_case`. Methods inside `#[php_impl_interface]` and items under
`#[derive(ZvalConvert)]` or `#[php_extern]` accept no `#[php]` option.

## `name` and `change_case`

The `name` option sets the PHP name of an item to a string literal. The
`change_case` option converts the Rust name to a different case. You can use
only one of the two on an item. If you use both, the macro gives a compile
error.

```rs
#[php(name = "NEW_NAME")]
#[php(change_case = "snake_case")]
```

The case is a string literal.

Available cases are:
- `snake_case`
- `PascalCase`
- `camelCase`
- `UPPER_CASE`
- `none` - No change
