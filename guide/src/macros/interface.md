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

Use the `extends` attribute to extend a built-in PHP interface or another Rust-defined interface.

For built-in PHP interfaces, use the explicit form:

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

For Rust-defined interfaces, you can use the simpler type syntax:

```rust,ignore
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

#[php_interface]
trait BaseInterface {
    fn base_method(&self) -> i32;
}

#[php_interface]
#[php(extends(BaseInterface))]
trait ExtendedInterface {
    fn extended_method(&self) -> String;
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

**Key feature**: The macro automatically registers the trait methods as PHP methods
on the class. You don't need to duplicate them in a separate `#[php_impl]` block.

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

    // Note: No need to add greet() here - it's automatically
    // registered by #[php_impl_interface] below
}

// Implement the interface for the class
// This automatically registers greet() as a PHP method
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

// The greet() method is available (registered by #[php_impl_interface])
echo $greeter->greet(); // Output: Hello, World!

// Can be used as type hint
function greet(Rust\Greetable $obj): void {
    echo $obj->greet();
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

### Example: Defining an Interface in a Library Crate

First, create a library crate that defines the interface:

```toml
# my-interfaces/Cargo.toml
[package]
name = "my-interfaces"
version = "0.1.0"

[dependencies]
ext-php-rs = "0.15"
```

```rust,no_run,ignore
// my-interfaces/src/lib.rs
use ext_php_rs::prelude::*;

/// A serializable interface that can convert objects to JSON.
#[php_interface]
#[php(name = "MyInterfaces\\Serializable")]
pub trait Serializable {
    fn to_json(&self) -> String;
}

// Re-export the generated PHP interface struct for consumers
pub use PhpInterfaceSerializable;
```

### Example: Implementing the Interface in Another Crate

Now create your extension crate that implements the interface:

```toml
# my-extension/Cargo.toml
[package]
name = "my-extension"
version = "0.1.0"

[lib]
crate-type = ["cdylib"]

[dependencies]
ext-php-rs = "0.15"
my-interfaces = { path = "../my-interfaces" }
```

```rust,no_run,ignore
// my-extension/src/lib.rs
use ext_php_rs::prelude::*;
use my_interfaces::Serializable;

#[php_class]
#[php(name = "MyExtension\\User")]
pub struct User {
    name: String,
    email: String,
}

#[php_impl]
impl User {
    pub fn __construct(name: String, email: String) -> Self {
        Self { name, email }
    }

    // Note: No need to add to_json() here - it's automatically
    // registered by #[php_impl_interface] below
}

// Register the interface implementation
// This automatically registers to_json() as a PHP method
#[php_impl_interface]
impl Serializable for User {
    fn to_json(&self) -> String {
        format!(r#"{{"name":"{}","email":"{}"}}"#, self.name, self.email)
    }
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        // Register the interface from the library crate
        .interface::<my_interfaces::PhpInterfaceSerializable>()
        .class::<User>()
}
```

### Using in PHP

```php
<?php

use MyExtension\User;
use MyInterfaces\Serializable;

$user = new User("John", "john@example.com");

// instanceof works across crates
assert($user instanceof Serializable);

// Type hints work
function serialize_object(Serializable $obj): string {
    return $obj->toJson();
}

echo serialize_object($user);
// Output: {"name":"John","email":"john@example.com"}
```

### Important Notes

1. **Automatic method registration**: The `#[php_impl_interface]` macro automatically
   registers all trait methods as PHP methods on the class. You don't need to duplicate
   them in a `#[php_impl]` block.

2. **Interface registration**: The interface must be registered in the `#[php_module]`
   function using `.interface::<PhpInterfaceName>()`.

3. **Link-time discovery**: The `inventory` crate uses link-time registration for
   interface discovery, so all implementations are automatically discovered when the
   final binary is linked.
