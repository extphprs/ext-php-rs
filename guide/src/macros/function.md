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

## Union, Intersection, and DNF Types

PHP 8.0+ supports union types (`int|string`), PHP 8.1+ supports intersection
types (`Countable&Traversable`), and PHP 8.2+ supports DNF (Disjunctive Normal
Form) types that combine both (`(Countable&Traversable)|ArrayAccess`).

You can declare these complex types using the `#[php(types = "...")]` attribute
on parameters. The parameter type should be `&Zval` since Rust cannot directly
represent these union/intersection types.

> **PHP Version Requirements for Internal Functions:**
>
> - **Primitive union types** (`int|string|null`) work on all PHP 8.x versions
> - **Intersection types** (`Countable&Traversable`) require **PHP 8.3+** for
>   reflection to show the correct type. On PHP 8.1-8.2, the type appears as
>   `mixed` via reflection, but function calls still work correctly.
> - **Class union types** (`Foo|Bar`) require **PHP 8.3+** for full support
> - **DNF types** require **PHP 8.3+** for full support
>
> This is a PHP limitation where internal (C extension) functions did not fully
> support intersection/DNF types until PHP 8.3. See
> [php-src#11969](https://github.com/php/php-src/pull/11969) for details.

### Union Types (PHP 8.0+)

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

/// Accepts int|string
#[php_function]
pub fn accept_int_or_string(#[php(types = "int|string")] value: &Zval) -> String {
    if let Some(i) = value.long() {
        format!("Got integer: {}", i)
    } else if let Some(s) = value.str() {
        format!("Got string: {}", s)
    } else {
        "Unknown type".to_string()
    }
}

/// Accepts float|bool|null
#[php_function]
pub fn accept_nullable(#[php(types = "float|bool|null")] value: &Zval) -> String {
    "ok".to_string()
}
# fn main() {}
```

### Intersection Types (PHP 8.1+)

Intersection types require a value to implement ALL specified interfaces:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

/// Accepts only objects implementing both Countable AND Traversable
#[php_function]
pub fn accept_countable_traversable(
    #[php(types = "Countable&Traversable")] value: &Zval,
) -> String {
    "ok".to_string()
}
# fn main() {}
```

### DNF Types (PHP 8.2+)

DNF (Disjunctive Normal Form) types combine union and intersection types.
Intersection groups must be wrapped in parentheses:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

/// Accepts (Countable&Traversable)|ArrayAccess
/// This accepts either:
/// - An object implementing both Countable AND Traversable, OR
/// - An object implementing ArrayAccess
#[php_function]
pub fn accept_dnf(
    #[php(types = "(Countable&Traversable)|ArrayAccess")] value: &Zval,
) -> String {
    "ok".to_string()
}

/// Multiple intersection groups: (Countable&Traversable)|(Iterator&ArrayAccess)
#[php_function]
pub fn accept_complex_dnf(
    #[php(types = "(Countable&Traversable)|(Iterator&ArrayAccess)")] value: &Zval,
) -> String {
    "ok".to_string()
}
# fn main() {}
```

### Union Types as Rust Enums (`PhpUnion`)

For a more ergonomic experience with union types, you can represent them as Rust
enums using the `#[derive(PhpUnion)]` macro. This allows you to use Rust's
pattern matching instead of manually checking the `Zval` type:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

/// Rust enum representing PHP `int|string`
#[derive(Debug, Clone, PhpUnion)]
enum IntOrString {
    Int(i64),
    Str(String),
}

/// Accepts int|string and processes it using Rust pattern matching
#[php_function]
fn process_value(#[php(union_enum)] value: IntOrString) -> String {
    match value {
        IntOrString::Int(n) => format!("Got integer: {}", n),
        IntOrString::Str(s) => format!("Got string: {}", s),
    }
}
# fn main() {}
```

The `#[derive(PhpUnion)]` macro:
- Generates `FromZval` and `IntoZval` implementations
- Tries each variant in order when converting from PHP values
- Provides `PhpUnion::union_types()` method returning the PHP types

Use `#[php(union_enum)]` on the parameter to tell the macro to use the
`PhpUnion` trait for type registration.

**Requirements:**
- Each enum variant must be a tuple variant with exactly one field
- The field type must implement `FromZval` and `IntoZval`
- No unit variants or named fields are allowed

#### Class and Interface Unions

For union types that include PHP classes or interfaces, use the `#[php(class = "...")]`
or `#[php(interface = "...")]` attributes on enum variants. These attributes override
the PHP type declaration for the variant:

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

/// Rust enum representing PHP `Iterator|int`
#[derive(PhpUnion)]
enum IteratorOrInt<'a> {
    #[php(interface = "Iterator")]
    Iter(&'a Zval),  // Use &Zval for object types
    Int(i64),
}

#[php_function]
fn process_iterator_or_int(#[php(union_enum)] value: IteratorOrInt) -> String {
    match value {
        IteratorOrInt::Iter(_) => "Got iterator".to_string(),
        IteratorOrInt::Int(n) => format!("Got int: {n}"),
    }
}
```

This generates PHP: `function process_iterator_or_int(Iterator|int $value): string`

**Variant Attributes:**
- `#[php(class = "ClassName")]` - For PHP class types
- `#[php(interface = "InterfaceName")]` - For PHP interface types
- `#[php(intersection = ["Interface1", "Interface2"])]` - For intersection types (PHP 8.2+ DNF)

> **Note:** For class/interface types, use `&Zval` or `&mut Zval` as the variant field
> type since owned `ZendObject` doesn't implement the required traits.

#### DNF Types (PHP 8.2+)

For PHP 8.2+ DNF (Disjunctive Normal Form) types that combine unions and intersections,
use the `#[php(intersection = [...])]` attribute:

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

/// Represents: (Countable&Traversable)|ArrayAccess
#[derive(PhpUnion)]
enum CountableTraversableOrArrayAccess<'a> {
    #[php(intersection = ["Countable", "Traversable"])]
    CountableTraversable(&'a Zval),
    #[php(interface = "ArrayAccess")]
    ArrayAccess(&'a Zval),
}

#[php_function]
fn process_dnf(#[php(union_enum)] _value: CountableTraversableOrArrayAccess) -> String {
    "ok".to_string()
}
```

This generates PHP: `function process_dnf((Countable&Traversable)|ArrayAccess $value): string`

> **Note:** When any variant uses `#[php(intersection = [...])]`, the macro
> automatically switches to DNF mode for all variants.

For complex object union types, consider using the macro-based syntax
`#[php(types = "...")]` which provides more flexibility:

### Using the Builder API

You can also create these types programmatically using the `FunctionBuilder` API:

```rust,ignore
use ext_php_rs::args::{Arg, TypeGroup};
use ext_php_rs::flags::DataType;

// Union of primitives
Arg::new_union("value", vec![DataType::Long, DataType::String]);

// Intersection type
Arg::new_intersection("value", vec!["Countable".to_string(), "Traversable".to_string()]);

// DNF type: (Countable&Traversable)|ArrayAccess
Arg::new_dnf("value", vec![
    TypeGroup::Intersection(vec!["Countable".to_string(), "Traversable".to_string()]),
    TypeGroup::Single("ArrayAccess".to_string()),
]);
```

## Returning `Result<T, E>`

You can also return a `Result` from the function. The error variant will be
translated into an exception and thrown. See the section on
[exceptions](../exceptions.md) for more details.
