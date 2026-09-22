# `#[php_enum]` Attribute

Enums can be exported to PHP as enums with the `#[php_enum]` attribute macro.
This attribute derives the `RegisteredClass` and `PhpEnum` traits on your enum.
To register the enum use the `enumeration::<EnumName>()` method on the `ModuleBuilder`
in the `#[php_module]` macro.

## Options

The `#[php_enum]` attribute can be configured with the following options:
- `#[php(name = "EnumName")]` or `#[php(change_case = "snake_case")]`: Sets the name of the enum in PHP.
  The default is the `PascalCase` name of the enum.
- `#[php(allow_native_discriminants)]`: Allows the use of native Rust discriminants (e.g., `Hearts = 1`).
- `#[php(change_cases_case = "snake_case")]`: Sets the rename rule for every case that has no `name` or
  `change_case` of its own. The old name of this option, `rename_cases`, is a compile error.

PHP enums are always public. `#[php(vis = "...")]` on an enum is a compile
error, and so is any argument passed to the macro itself, such as
`#[php_enum(name = "Suit")]`.

The cases of the enum can be configured with the following options:
- `#[php(name = "CaseName")]` or `#[php(change_case = "snake_case")]`: Sets the name of the enum case in PHP.
  The default is the `PascalCase` name of the case.
- `#[php(value = "value")]` or `#[php(value = 123)]`: Sets the discriminant value for the enum case.
  This can be a string or an integer. If not set, the case will be exported as a simple enum case without a discriminant.

### Example

This example creates a PHP enum `Suit`.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

#[php_enum]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.enumeration::<Suit>()
}
# fn main() {}
```

## Backed Enums
Enums can also be backed by either `i64` or `&'static str`. Those values can be set using the
`#[php(value = "value")]` or `#[php(value = 123)]` attributes on the enum variants.

All variants must have a value of the same type, either all `i64` or all `&'static str`.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

#[php_enum]
pub enum Suit {
    #[php(value = "hearts")]
    Hearts,
    #[php(value = "diamonds")]
    Diamonds,
    #[php(value = "clubs")]
    Clubs,
    #[php(value = "spades")]
    Spades,
}
#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.enumeration::<Suit>()
}
# fn main() {}
```

### 'Native' Discriminators
Native rust discriminants are currently not supported and will not be exported to PHP.

To avoid confusion a compiler error will be raised if you try to use a native discriminant.
You can ignore this error by adding the `#[php(allow_native_discriminants)]` attribute to your enum.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

#[php_enum]
#[php(allow_native_discriminants)]
pub enum Suit {
    Hearts = 1,
    Diamonds = 2,
    Clubs = 3,
    Spades = 4,
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.enumeration::<Suit>()
}
# fn main() {}
```


TODO: Add backed enums example
