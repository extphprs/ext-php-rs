# `#[php_impl]` Attribute

You can export an entire `impl` block to PHP. This exports all methods as well
as constants to PHP on the class that it is implemented on. This requires the
`#[php_class]` macro to already be used on the underlying struct. Trait
implementations cannot be exported to PHP. Only one `impl` block can be exported
per class.

If you do not want a function exported to PHP, you should place it in a separate
`impl` block.

If you want to use async Rust, use `#[php_async_impl]`, instead: see [here &raquo;](./async_impl.md) for more info.

## Options

By default all constants are renamed to `UPPER_CASE` and all methods are renamed to
camelCase. This can be changed by passing the `change_method_case` and
`change_constant_case` as `#[php]` attributes on the `impl` block. The options are:

- `#[php(change_method_case = "snake_case")]` - Renames the method to snake case.
- `#[php(change_constant_case = "snake_case")]` - Renames the constant to snake case.

See the [`name` and `change_case`](./php.md#name-and-change_case) section for a list of all
available cases.

## Methods

Methods basically follow the same rules as functions, so read about the
[`php_function`] macro first. The primary difference between functions and
methods is they are bounded by their class object.

Class methods can take a `&self` or `&mut self` parameter. They cannot take a
consuming `self` parameter. Static methods can omit this `self` parameter.

To access the underlying Zend object, you can take a reference to a
`ZendClassObject<T>` in place of the self parameter, where the parameter must
be named `self_`. This can also be used to return a reference to `$this`.

The rest of the options are passed as separate attributes:

- `#[php(defaults(i = 5, b = "hello"))]` - Sets the default value for parameter(s).
- `#[php(optional = i)]` - Sets the first optional parameter. Note that this also sets
  the remaining parameters as optional, so all optional parameters must be a
  variant of `Option<T>`.
- `#[php(vis = "public")]`, `#[php(vis = "protected")]` and `#[php(vis = "private")]` - Sets the visibility of the
  method.
- `#[php(name = "method_name")]` - Renames the PHP method to a different identifier,
  without renaming the Rust method name.

The `#[php(defaults)]` and `#[php(optional)]` attributes operate the same as the
equivalent function attribute parameters.

### Static Methods

Methods that do not take a `&self` or `&mut self` parameter are automatically
exported as static methods. These can be called on the class itself without
creating an instance.

```rust,ignore
#[php_impl]
impl MyClass {
    // Static method - no self parameter
    pub fn create_default() -> Self {
        Self { /* ... */ }
    }

    // Instance method - takes &self
    pub fn get_value(&self) -> i32 {
        self.value
    }
}
```

From PHP:

```php
$obj = MyClass::createDefault(); // Static call
$val = $obj->getValue();         // Instance call
```

### Constructors

By default, if a class does not have a constructor, it is not constructable from
PHP. It can only be returned from a Rust function to PHP.

Constructors are Rust methods which can take any amount of parameters and
returns either `Self` or `Result<Self, E>`, where `E: Into<PhpException>`. When
the error variant of `Result` is encountered, it is thrown as an exception and
the class is not constructed.

Constructors are designated by either naming the method `__construct` or by
annotating a method with the `#[php(constructor)]` attribute. Note that when using
the attribute, the function is not exported to PHP like a regular method.

Constructors cannot use the visibility or rename attributes listed above.

## Constants

Constants are defined as regular Rust `impl` constants. Any type that implements
`IntoZval` can be used as a constant. Constant visibility is not supported at
the moment, and therefore no attributes are valid on constants.

## Property getters and setters

You can add properties to classes which use Rust functions as getters and/or
setters. This is done with the `#[php(getter)]` and `#[php(setter)]` attributes. By
default, the `get_` or `set_` prefix is trimmed from the start of the function
name, and the remainder is used as the property name.

If you want to use a different name for the property, you can pass a `name` or
`change_case` option to the `#[php]` attribute which will change the property name.

Properties do not necessarily have to have both a getter and a setter, if the
property is immutable the setter can be omitted, and vice versa for getters.

The `#[php(getter)]` and `#[php(setter)]` attributes are mutually exclusive on methods.
Properties cannot have multiple getters or setters, and the property name cannot
conflict with field properties defined on the struct.

As the same as field properties, method property types must implement both
`IntoZval` and `FromZval`.

## Example

Continuing on from our `Human` example in the structs section, we will define a
constructor, as well as getters for the properties. We will also define a
constant for the maximum age of a `Human`.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{prelude::*, types::ZendClassObject};

#[php_class]
#[derive(Debug, Default)]
pub struct Human {
    name: String,
    age: i32,
    #[php(prop)]
    address: String,
}

#[php_impl]
impl Human {
    const MAX_AGE: i32 = 100;

    // No `#[constructor]` attribute required here - the name is `__construct`.
    pub fn __construct(name: String, age: i32) -> Self {
        Self {
            name,
            age,
            address: String::new()
        }
    }

    #[php(getter)]
    pub fn get_name(&self) -> String {
        self.name.to_string()
    }

    #[php(setter)]
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    #[php(getter)]
    pub fn get_age(&self) -> i32 {
        self.age
    }

    pub fn introduce(&self) {
        println!("My name is {} and I am {} years old. I live at {}.", self.name, self.age, self.address);
    }

    pub fn get_raw_obj(self_: &mut ZendClassObject<Human>) -> &mut ZendClassObject<Human> {
        dbg!(self_)
    }

    pub fn get_max_age() -> i32 {
        Self::MAX_AGE
    }
}
#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.class::<Human>()
}
# fn main() {}
```

Using our newly created class in PHP:

```php
<?php

$me = new Human('David', 20);

$me->introduce(); // My name is David and I am 20 years old.
var_dump(Human::get_max_age()); // int(100)
var_dump(Human::MAX_AGE); // int(100)
```

[`php_async_impl`]: ./async_impl.md
