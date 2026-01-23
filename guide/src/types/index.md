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

## Complex Type Declarations

For parameters that need PHP's advanced type system features (union types,
intersection types, or DNF types), you can use `&Zval` as the parameter type
with the `#[php(types = "...")]` attribute:

- **Union types** (PHP 8.0+): `#[php(types = "int|string")]`
- **Intersection types** (PHP 8.1+): `#[php(types = "Countable&Traversable")]`
- **DNF types** (PHP 8.2+): `#[php(types = "(Countable&Traversable)|ArrayAccess")]`

See the [function macro documentation](../macros/function.md#union-intersection-and-dnf-types)
for detailed examples.
