# `#[php_function]` Attribute

Used to annotate functions which should be exported to PHP. Note that this
should not be used on class methods - see the `#[php_impl]` macro for that.

See the [list of types](../types/index.md) that are valid as parameter and
return types.

A function accepts the `name`, `change_case`, `defaults` and `optional`
options. PHP functions have no visibility, so `#[php(vis = "...")]` is a
compile error on a function.

## Optional parameters

An `Option<T>` parameter accepts `null` and can be omitted. The macro reads this
from the type through `FromZvalMut::NULLABLE`, so a type alias such as
`type MaybeAge = Option<i64>` or the qualified path `std::option::Option<i64>`
behaves like `Option<i64>`. The trailing run of parameters that are nullable or
have a default value is optional.

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
default, it does not need to be a variant of `Option`. The default expression is
converted into the parameter type with `Into`, and the type must implement
`StubLiteral`, which every scalar, string, `Option`, `Vec` and `HashMap` does.
The module renders the value once at load time into the stub file and into
`ReflectionParameter::getDefaultValue()`. Each key in `defaults` must name a
parameter of the function. If a key names no parameter, the build fails:

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
arguments before optional arguments. This is done through the `optional`
attribute option, which names the first optional parameter. PHP callers can
omit that parameter and every parameter after it. Each of these parameters
must be an `Option<T>`, have a default, or be the variadic `&[T]` tail.
Otherwise the build fails with an error on the type of that parameter. If
`optional` names no parameter, the build also fails:

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

A slice parameter `&[T]` in last position is variadic, the equivalent of the PHP
`...$args` syntax. `T` is any argument type: `&[&Zval]` receives the raw values,
`&[i64]` receives converted integers. A variadic parameter that is not the last
parameter is a compile error. The slice must be spelled in the signature; a type
alias that hides the slice is not detected.

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

## Performance

The `#[php_function]` macro generates a zero-allocation fast path that reads
arguments directly from the PHP call frame, matching how native C extensions
parse parameters. This applies automatically — no configuration needed.

The same fast path is used for class methods defined with `#[php_impl]`,
including instance methods (`&self`, `&mut self`) and static methods.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
// Fast path: standalone functions
#[php_function]
pub fn fast(name: String, age: Option<i32>) -> String { name }

// Fast path: parameters with defaults
#[php_function]
#[php(defaults(offset = 0))]
pub fn also_fast(haystack: &str, offset: i64) -> i64 { offset }
# fn main() {}
```

```rust,ignore
#[php_impl]
impl MyClass {
    // Fast path: static methods
    pub fn create(n: i32) -> Self { /* ... */ }

    // Fast path: instance methods
    pub fn get_value(&self, offset: i32) -> i32 { /* ... */ }
}
```

The only case that falls back to the runtime argument parser is when using
variadic arguments (`&[T]`, the Rust equivalent of PHP's `...$args`):

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::{prelude::*, types::Zval};
// Slower path: variadic arguments require runtime parsing
#[php_function]
pub fn add(first: u32, rest: &[&Zval]) -> u32 { first }
# fn main() {}
```

## Returning `Result<T, E>`

You can also return a `Result` from the function. The error variant will be
translated into an exception and thrown. See the section on
[exceptions](../exceptions.md) for more details.
