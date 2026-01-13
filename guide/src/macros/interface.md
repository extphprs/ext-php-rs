# `#[php_interface]` Attribute

You can export a `Trait` block to PHP. This exports all methods as well as
constants to PHP on the interface. Trait method SHOULD NOT contain default
implementations, as these are not supported in PHP interfaces.

## Options

By default all constants are renamed to `UPPER_CASE` and all methods are renamed to
`camelCase`. This can be changed by passing the `change_method_case` and
`change_constant_case` as `#[php]` attributes on the `impl` block. The options are:

- `#[php(change_method_case = "snake_case")]` - Renames the method to snake case.
- `#[php(change_constant_case = "snake_case")]` - Renames the constant to snake case.

See the [`name` and `change_case`](./php.md#name-and-change_case) section for a list of all
available cases.

## Methods

See the [`php_impl`](./impl.md#)

## Constants

See the [`php_impl`](./impl.md#)

## Example

Define an example trait with methods and constant:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{prelude::*, types::ZendClassObject};


#[php_interface]
#[php(name = "Rust\\TestInterface")]
trait Test {
    const TEST: &'static str = "TEST";

    fn co();

    #[php(defaults(value = 0))]
    fn set_value(&mut self, value: i32);
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .interface::<PhpInterfaceTest>()
}

# fn main() {}
```

Using our newly created interface in PHP:

```php
<?php

assert(interface_exists("Rust\TestInterface"));

class B implements Rust\TestInterface {

    public static function co() {}

    public function setValue(?int $value = 0) {

    }
}

```

## Interface Inheritance

PHP interfaces can extend other interfaces. You can achieve this in two ways:

### Using `#[php(extends(...))]`

Use the `extends` attribute to extend a built-in PHP interface or another interface:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::ce;

#[php_interface]
#[php(extends(ce = ce::throwable, stub = "\\Throwable"))]
#[php(name = "MyException")]
trait MyExceptionInterface {
    fn get_error_code(&self) -> i32;
}

# fn main() {}
```

### Using Rust Trait Bounds

You can also use Rust's trait bound syntax. When a trait marked with `#[php_interface]`
has supertraits, the PHP interface will automatically extend those parent interfaces:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

#[php_interface]
#[php(name = "Rust\\ParentInterface")]
trait ParentInterface {
    fn parent_method(&self) -> String;
}

// ChildInterface extends ParentInterface in PHP
#[php_interface]
#[php(name = "Rust\\ChildInterface")]
trait ChildInterface: ParentInterface {
    fn child_method(&self) -> String;
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .interface::<PhpInterfaceParentInterface>()
        .interface::<PhpInterfaceChildInterface>()
}

# fn main() {}
```

In PHP:

```php
<?php

// ChildInterface extends ParentInterface
assert(is_a('Rust\ChildInterface', 'Rust\ParentInterface', true));
```

# `#[php_impl_interface]` Attribute

The `#[php_impl_interface]` attribute allows a Rust class to implement a custom PHP
interface defined with `#[php_interface]`. This creates a relationship where PHP's
`instanceof` and `is_a()` recognize the implementation.

## Example

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

// Define a custom interface
#[php_interface]
#[php(name = "Rust\\Greetable")]
trait Greetable {
    fn greet(&self) -> String;
}

// Define a class
#[php_class]
#[php(name = "Rust\\Greeter")]
pub struct Greeter {
    name: String,
}

#[php_impl]
impl Greeter {
    pub fn __construct(name: String) -> Self {
        Self { name }
    }
}

// Implement the interface for the class
#[php_impl_interface]
impl Greetable for Greeter {
    fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .interface::<PhpInterfaceGreetable>()
        .class::<Greeter>()
}

# fn main() {}
```

Using in PHP:

```php
<?php

$greeter = new Rust\Greeter("World");

// instanceof works
assert($greeter instanceof Rust\Greetable);

// is_a() works
assert(is_a($greeter, 'Rust\Greetable'));

// Can be used as type hint
function greet(Rust\Greetable $obj): void {
    // $obj->greet() method is available
}

greet($greeter);
```

## When to Use

- Use `#[php_impl_interface]` for custom interfaces you define with `#[php_interface]`
- Use `#[php(implements(ce = ...))]` on `#[php_class]` for built-in PHP interfaces
  like `Iterator`, `ArrayAccess`, `Countable`, etc.

See the [Classes documentation](./classes.md#implementing-an-interface) for examples
of implementing built-in interfaces.

## Cross-Crate Support

The `#[php_impl_interface]` macro supports cross-crate interface discovery via the
[`inventory`](https://crates.io/crates/inventory) crate. This means you can define
an interface in one crate and implement it in another crate, and the implementation
will be automatically discovered at link time.
