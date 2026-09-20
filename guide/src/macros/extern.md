# `#[php_extern]` Attribute

Attribute used to annotate `extern` blocks which are deemed as PHP
functions.

This allows you to 'import' PHP functions into Rust so that they can be
called like regular Rust functions. Parameters can be any type that
implements [`IntoZval`], and the return type can be anything that implements
[`From<Zval>`] (notice how [`Zval`] is consumed rather than borrowed in this
case).

Unlike most other attributes, this does not need to be placed inside a
`#[php_module]` block.

# Panics

The function can panic when called under a few circumstances:

* The function could not be found or was not callable.
* One of the parameters could not be converted into a [`Zval`].
* The actual function call failed internally.
* The output [`Zval`] could not be parsed into the output type.

Inside a function that PHP calls, these panics become a PHP `Error`. The
process does not abort. See [Exceptions](../exceptions.md#panics).

The last point can be important when interacting with functions that return
unions, such as [`strpos`] which can return an integer or a boolean. In this
case, a [`Zval`] should be returned as parsing a boolean to an integer is
invalid, and vice versa.

# Example

This `extern` block imports the [`strpos`] function from PHP. Notice that
the string parameters can take either [`String`] or [`&str`], the optional
parameter `offset` is an [`Option<i64>`], and the return value is a [`Zval`]
as the return type is an integer-boolean union.

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{
    prelude::*,
    types::Zval,
};

#[php_extern]
extern "C" {
    fn strpos(haystack: &str, needle: &str, offset: Option<i64>) -> Zval;
}

#[php_function]
pub fn my_strpos() {
    assert_eq!(unsafe { strpos("Hello", "e", None) }.long(), Some(1));
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(my_strpos))
}
# fn main() {}
```

[`strpos`]: https://www.php.net/manual/en/function.strpos.php
[`IntoZval`]: crate::convert::IntoZval
[`Zval`]: crate::types::Zval
