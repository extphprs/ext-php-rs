# `Option<T>`

Options are used for optional and nullable parameters, as well as null returns.
It is valid to be converted to/from a zval as long as the underlying `T` generic
is also able to be converted to/from a zval.

| `T` parameter | `&T` parameter | `T` Return type | `&T` Return type | PHP representation                 |
| ------------- | -------------- | --------------- | ---------------- | ---------------------------------- |
| Yes           | No             | Yes             | No               | Depends on `T`, `null` for `None`. |

An `Option<T>` parameter is nullable. If the caller passes `null`, the function
receives `None`. A trailing `Option<T>` parameter is also optional. If the caller
omits it, the function receives `None`, and PHP reflection reports a default of
`null`. The macro reads this from `FromZvalMut::NULLABLE`, so a type alias or a
qualified path behaves the same way as `Option<T>`.

Returning `Option<T>` is a nullable return type. Returning `None` will return
null to PHP.

## Rust example

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
#[php_function]
pub fn test_option_null(input: Option<String>) -> Option<String> {
    input.map(|input| format!("Hello {}", input).into())
}
# fn main() {}
```

## PHP example

```php
<?php

var_dump(test_option_null("World")); // string(11) "Hello World"
var_dump(test_option_null()); // null
```
