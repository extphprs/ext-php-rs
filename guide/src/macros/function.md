# `#[php_function]` Attribute

Used to annotate functions which should be exported to PHP. Note that this
should not be used on class methods - see the `#[php_impl]` macro for that.

See the [list of types](../types/index.md) that are valid as parameter and
return types.

## Optional parameters

Optional parameters can be used by setting the Rust parameter type to a variant
of `Option<T>`. The macro will then figure out which parameters are optional by
using the last consecutive arguments that are a variant of `Option<T>` or have a
default value.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

#[php_function]
pub fn greet(name: String, age: Option<i32>) -> String {
    let mut greeting = format!("Hello, {}!", name);

    if let Some(age) = age {
        greeting += &format!(" You are {} years old.", age);
    }

    greeting
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(greet))
}
# fn main() {}
```

Default parameter values can also be set for optional parameters. This is done
through the `#[php(defaults)]` attribute option. When an optional parameter has a
default, it does not need to be a variant of `Option`:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

#[php_function]
#[php(defaults(offset = 0))]
pub fn rusty_strpos(haystack: &str, needle: &str, offset: i64) -> Option<usize> {
    let haystack: String = haystack.chars().skip(offset as usize).collect();
    haystack.find(needle)
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(rusty_strpos))
}
# fn main() {}
```

Note that if there is a non-optional argument after an argument that is a
variant of `Option<T>`, the `Option<T>` argument will be deemed a nullable
argument rather than an optional argument.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

/// `age` will be deemed required and nullable rather than optional.
#[php_function]
pub fn greet(name: String, age: Option<i32>, description: String) -> String {
    let mut greeting = format!("Hello, {}!", name);

    if let Some(age) = age {
        greeting += &format!(" You are {} years old.", age);
    }

    greeting += &format!(" {}.", description);
    greeting
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(greet))
}
# fn main() {}
```

You can also specify the optional arguments if you want to have nullable
arguments before optional arguments. This is done through an attribute
parameter:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

/// `age` will be deemed required and nullable rather than optional,
/// while description will be optional.
#[php_function]
#[php(optional = "description")]
pub fn greet(name: String, age: Option<i32>, description: Option<String>) -> String {
    let mut greeting = format!("Hello, {}!", name);

    if let Some(age) = age {
        greeting += &format!(" You are {} years old.", age);
    }

    if let Some(description) = description {
        greeting += &format!(" {}.", description);
    }

    greeting
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(greet))
}
# fn main() {}
```

## Variadic Functions

Variadic functions can be implemented by specifying the last argument in the Rust
function to the type `&[&Zval]`. This is the equivalent of a PHP function using
the `...$args` syntax.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{prelude::*, types::Zval};

/// This can be called from PHP as `add(1, 2, 3, 4, 5)`
#[php_function]
pub fn add(number: u32, numbers:&[&Zval]) -> u32 {
    // numbers is a slice of 4 Zvals all of type long
    number
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(add))
}
# fn main() {}
```

## Returning `Result<T, E>`

You can also return a `Result` from the function. The error variant will be
translated into an exception and thrown. See the section on
[exceptions](../exceptions.md) for more details.

## Raw Functions

For performance-critical code, you can use the `#[php(raw)]` attribute to bypass
all argument parsing and type conversion overhead. Raw functions receive direct
access to the `ExecuteData` and return `Zval`, giving you complete control over
argument handling.

This is useful when:
- You need maximum performance and want to avoid allocation overhead
- You want to handle variadic arguments manually
- You need direct access to the PHP execution context

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;

#[php_function]
#[php(raw)]
pub fn fast_add(ex: &mut ExecuteData, retval: &mut Zval) {
    // Get arguments directly without type conversion overhead
    let a = unsafe { ex.get_arg(0) }
        .and_then(|zv| zv.long())
        .unwrap_or(0);
    let b = unsafe { ex.get_arg(1) }
        .and_then(|zv| zv.long())
        .unwrap_or(0);

    retval.set_long(a + b);
}

/// Sum all arguments passed to the function
#[php_function]
#[php(raw)]
pub fn sum_all(ex: &mut ExecuteData, retval: &mut Zval) {
    let num_args = ex.num_args();
    let mut sum: i64 = 0;

    for i in 0..num_args {
        if let Some(zv) = unsafe { ex.get_arg(i as usize) } {
            sum += zv.long().unwrap_or(0);
        }
    }

    retval.set_long(sum);
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .function(wrap_function!(fast_add))
        .function(wrap_function!(sum_all))
}
# fn main() {}
```

### `ExecuteData` Methods

When using raw functions, you have access to these `ExecuteData` methods:

- `num_args()` - Returns the number of arguments passed to the function
- `get_arg(n)` - Returns a reference to the argument at index `n` (0-based). This is an unsafe method; the caller must ensure `n < num_args()`

Raw functions bypass the standard argument parser, so you are responsible for
validating argument count and types yourself.
