# `Binary Slices`

Binary data is represented as a string in PHP. The most common source of this
data is from the [`pack`] and [`unpack`] functions. It allows you to use a PHP
string as a read-only slice in Rust.

| `T` parameter | `&T` parameter | `T` Return type | `&T` Return type | PHP representation |
| ------------- | -------------- | --------------- | ---------------- | ------------------ |
| Yes           | No             | No              | No               | `zend_string`      |

The binary type is represented as a string in PHP. Although not encoded, the
data is converted into a slice and then the pointer to the data is set as the
string pointer, with the length of the array being the length of the string.

`BinarySlice<T>` is valid when `T` implements `PackSlice`. This is currently
implemented on most primitive numbers (i8, i16, i32, i64, u8, u16, u32, u64,
isize, usize, f32, f64).

The conversion fails (the argument is rejected) when the string length is not a
whole multiple of `size_of::<T>()` or when the string bytes are not aligned for
`T`. Zend allocates strings on 8-byte boundaries, so the alignment check only
fails for exotic allocators. `PackSlice::unpack_into` itself is `unsafe`; use
`Zval::binary_slice`, which performs both checks.

[`pack`]: https://www.php.net/manual/en/function.pack.php
[`unpack`]: https://www.php.net/manual/en/function.unpack.php

## Rust Usage

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::binary_slice::BinarySlice;

#[php_function]
pub fn test_binary_slice(input: BinarySlice<u8>) -> u8 {
    let mut sum = 0;
    for i in input.iter() {
        sum += i;
    }

    sum
}
# fn main() {}
```

## PHP Usage

```php
<?php

$data = pack('C*', 1, 2, 3, 4, 5);
$output = test_binary_slice($data);
var_dump($output); // 15
```
