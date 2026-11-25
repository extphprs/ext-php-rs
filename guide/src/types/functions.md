# Functions & methods

PHP functions and methods are represented by the `Function` struct.

You can use the `try_from_function` and `try_from_method` methods to obtain a Function struct corresponding to the passed function or static method name.
It's heavily recommended you reuse returned `Function` objects, to avoid the overhead of looking up the function/method name.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

use ext_php_rs::zend::Function;

#[php_function]
pub fn test_function() -> () {
    let var_dump = Function::try_from_function("var_dump").unwrap();
    let _ = var_dump.try_call(vec![&"abc"]);
}

#[php_function]
pub fn test_method() -> () {
    let f = Function::try_from_method("ClassName", "staticMethod").unwrap();
    let _ = f.try_call(vec![&"abc"]);
}

# fn main() {}
```

## Untyped and union parameter types

`#[php_function]` declares each parameter with the PHP type of its Rust type.
A function built with `FunctionBuilder` can also declare a parameter without a
type, as `function f($value)` does, with `Arg::untyped`, and a union of scalar
types with `Arg::or`. A return type becomes a union with
`FunctionBuilder::or_returns`. Reflection reports these types, and PHP's
inheritance checks compare a method's overrides against them.

```rust,ignore
use ext_php_rs::args::Arg;
use ext_php_rs::builders::FunctionBuilder;
use ext_php_rs::flags::DataType;

// function check($value) {}
let check = FunctionBuilder::new("check", check_handler).arg(Arg::untyped("value"));

// function twice(int|float $value): int|float {}
let twice = FunctionBuilder::new("twice", twice_handler)
    .arg(Arg::new("value", DataType::Long).or(DataType::Double))
    .returns(DataType::Long, false, false)
    .or_returns(DataType::Double);
```

A union joins `bool`, `int`, `float`, `string`, `array`, `object` and `null`;
any other type (a class, `iterable`, `resource`, `mixed`, ...), a union on an
untyped argument, or `or_returns` without `returns` fails `FunctionBuilder::build`
with `Error::InvalidUnionType`. The handler's type, `FunctionHandler`, is
exported from `ext_php_rs::builders`.

These types are what reflection and inheritance checks see. Release builds of
PHP do not check an internal function's arguments against them: the handler
receives whatever the caller passed and must check it itself. Debug builds of
PHP do check the return value against the declared return type and assert on a
mismatch, so a handler declared `: int|float` must not return `null`, even on
bad input.
