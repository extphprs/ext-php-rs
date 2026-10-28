# Class Object

A class object is an instance of a Rust struct (which has been registered as a
PHP class) that has been allocated alongside an object. You can think of a class
object as a superset of an object, as a class object contains a Zend object.

| `T` parameter | `&T` parameter        | `T` Return type | `&T` Return type      | PHP representation             |
| ------------- | --------------------- | --------------- | --------------------- | ------------------------------ |
| No            | `&ZendClassObject<T>` | Yes             | `&ZendClassObject<T>` | Zend object and a Rust struct. |

To get the Rust struct, use `try_borrow()` for shared access or
`try_borrow_mut()` for exclusive access.

## Examples

### Returning a reference to `self`

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{prelude::*, types::ZendClassObject};

#[php_class]
pub struct Example {
    foo: i32,
    bar: i32
}

#[php_impl]
impl Example {
    // ext-php-rs treats the method as associated due to the `self_` argument.
    // The argument _must_ be called `self_`.
    pub fn builder_pattern(
        self_: &ZendClassObject<Example>,
    ) -> &ZendClassObject<Example> {
        // do something with `self_`
        self_
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.class::<Example>()
}
# fn main() {}
```

### Creating a new class instance

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

#[php_class]
pub struct Example {
    foo: i32,
    bar: i32
}

#[php_impl]
impl Example {
    pub fn make_new(foo: i32, bar: i32) -> Example {
        Example { foo, bar }
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.class::<Example>()
}
# fn main() {}
```
