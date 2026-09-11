# Exceptions

Exceptions can be thrown from Rust to PHP. The inverse (catching a PHP exception
in Rust) is currently being worked on.

## Throwing exceptions

[`PhpException`] is the type that represents an exception. It contains the
message contained in the exception, the type of exception and a status code to
go along with the exception.

You can create a new exception with the `new()`, `from_message()`, or
`from_class::<T>()` methods. `Into<PhpException>` is implemented for `String`
and `&str`, which creates an exception of the type `Exception` with a code of 0.
It may be useful to implement `Into<PhpException>` for your error type.

Calling the `throw()` method on a `PhpException` attempts to throw the exception
in PHP. This function can fail if the type of exception is invalid (i.e. does
not implement `Exception` or `Throwable`). Upon success, nothing will be
returned.

`throw()` does nothing when an exception is already pending in the engine: that
one keeps propagating with its own class and stack trace. This is what lets a
PHP exception caught by `ZendCallable::try_call` reach the caller unchanged even
though the Rust side reports it as `Error::ExceptionPending`.

To look at the pending exception from Rust, read its class with
`ExecutorGlobals::pending_exception_class()`, or borrow the object through the
globals guard, which must outlive the borrow:

```rust,ignore
let globals = ExecutorGlobals::get();
if let Some(exception) = globals.exception() {
    // inspect it; it stays owned by the engine
}
```

To handle it yourself, take ownership with `ExecutorGlobals::take_exception()`:
the engine then stops propagating it and rethrowing becomes your
responsibility.

`IntoZval` is also implemented for `Result<T, E>`, where `T: IntoZval` and
`E: Into<PhpException>`. If the result contains the error variant, the exception
is thrown. This allows you to return a result from a PHP function annotated with
the `#[php_function]` attribute.

### Examples

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use std::convert::TryInto;

// Trivial example - PHP represents all integers as `u64` on 64-bit systems
// so the `u32` would be converted back to `u64`, but that's okay for an example.
#[php_function]
pub fn something_fallible(n: u64) -> PhpResult<u32> {
    let n: u32 = n.try_into().map_err(|_| "Could not convert into u32")?;
    Ok(n)
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
}
# fn main() {}
```

[`PhpException`]: https://docs.rs/ext-php-rs/0.5.0/ext_php_rs/php/exceptions/struct.PhpException.html
