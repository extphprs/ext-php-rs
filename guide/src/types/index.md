# Types

In PHP, data is stored in containers called zvals (zend values). Internally,
these are effectively tagged unions (enums in Rust) without the safety that Rust
introduces. Passing data between Rust and PHP requires the data to become a
zval. This is done through two traits: `FromZval` and `IntoZval`. These traits
have been implemented on most regular Rust types:

- Primitive integers (`i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`,
  `usize`, `isize`).
- Double and single-precision floating point numbers (`f32`, `f64`).
- Booleans.
- Strings (`String` and `&str`)
- `Vec<T>` where T implements `IntoZval` and/or `FromZval`.
- `HashMap<String, T>` where T implements `IntoZval` and/or `FromZval`.
- `Binary<T>` where T implements `Pack`, used for transferring binary string
  data.
- `BinarySlice<T>` where T implements `Pack`, used for exposing PHP binary
  strings as read-only slices.
- A PHP callable closure or function wrapped with `Callable`.
- `Option<T>` where T implements `IntoZval` and/or `FromZval`, and where `None`
  is converted to a PHP `null`.

Return types can also include:

- Any class type which implements `RegisteredClass` (i.e. any struct you have
  registered with PHP).
- An immutable reference to `self` when used in a method, through the `ClassRef`
  type.
- A Rust closure wrapped with `Closure`.
- `Result<T, E>`, where `T: IntoZval` and `E: Into<PhpException>`. When the
  error variant is encountered, it is converted into a `PhpException` and thrown
  as an exception.

For a type to be returnable, it must implement `IntoZval`, while for it to be
valid as a parameter, it must implement `FromZval`.

## Compound PHP types

`int|string`, `Foo|Bar`, `Countable&Traversable`, and `(A&B)|C` are all
expressible at the `Arg` and `FunctionBuilder` layer through the [`PhpType`]
enum. Two ergonomic paths surface this on `#[php_function]` and
`#[php_impl]` signatures:

- The [`#[php(types = "...")]`](../macros/php.md) attribute, which takes a
  PHP type string and parses it at macro-expansion time — invalid syntax
  becomes a `compile_error!` spanned on the literal.
- The [`#[derive(PhpUnion)]`](../macros/php_union.md) macro, which lets you
  model a union as a Rust enum and have the macro infer the registered shape
  from the variants.

[`PhpType`]: https://docs.rs/ext-php-rs/latest/ext_php_rs/types/enum.PhpType.html
