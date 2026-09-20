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

## Panics

A Rust panic never crosses the `extern "C"` boundary into PHP. The crate catches
it at every entry point that it generates or installs:

- Functions, methods, static methods, constructors and `#[php_impl_interface]`
  methods, through `zend::run_handler`.
- `Closure` calls.
- Property getters and setters, `isset()`, the `(array)` cast, `var_export` on a
  Rust-backed object, and `clone`.

When the panic reaches the boundary, the unwind already ran the destructors of
every Rust frame inside the handler. The crate then throws a PHP `Error`
exception and the handler returns normally. The message starts with
`Rust panic: ` and continues with the panic message. When the payload is not a
string, for example after `panic_any(42)`, the message continues with
`non-string panic payload`. A `catch (\Exception $e)` block does not catch this
`Error`. A `catch (\Error $e)` or `catch (\Throwable $e)` block does. The
process does not stop, and the default panic hook still writes the location and
the message to stderr.

If an exception is already pending when the panic happens, that exception
continues to propagate. The panic then only reaches the panic hook. This is the
rule that `throw()` also follows.

If `Drop` of a Rust-backed object panics, the crate reports an `E_WARNING` with
the message `Rust panic in Drop for <Class>: ...` and frees the object. No
exception is possible there, because the destructor also runs from the garbage
collector and at request shutdown.

The following cases still stop the process:

- A second panic while the first one unwinds, for example a `Drop` that panics.
  Rust turns this into an abort.
- A build profile with `panic = "abort"`. Nothing is caught.
- The hooks that the crate installs outside of a PHP call: the observer, the
  zend extension, the error and exception observers, the module globals
  constructor and destructor, and the embed `Sapi` trampolines. The engine is in
  the middle of an operation there, so an exception or a bailout is not safe.
- An `extern "C"` function that you give to the engine yourself: `info_function`,
  the request startup and shutdown functions, `post_deactivate_function`, the
  `startup` function of `#[php_module]`, stream wrapper operations, and a
  handler that you write by hand for `FunctionBuilder::new`.

If you write a request-time handler by hand, run its body through
`zend::run_handler`. The handler then behaves like generated code:

```rust,ignore
use ext_php_rs::zend::{ExecuteData, run_handler};
use ext_php_rs::types::Zval;
use std::panic::AssertUnwindSafe;

extern "C" fn my_handler(ex: &mut ExecuteData, retval: &mut Zval) {
    run_handler(AssertUnwindSafe(|| {
        // This code can panic, bail out or throw.
    }));
}
```

A `Mutex` in your own statics stays poisoned after a caught panic, as in any
Rust program.

[`PhpException`]: https://docs.rs/ext-php-rs/0.5.0/ext_php_rs/php/exceptions/struct.PhpException.html
