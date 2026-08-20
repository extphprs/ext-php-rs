# `#[php_module]` Attribute

The module macro is used to annotate the `get_module` function, which is used by
the PHP interpreter to retrieve information about your extension, including the
name, version, functions and extra initialization functions. Regardless if you
use this macro, your extension requires a `extern "C" fn get_module()` so that
PHP can get this information.

The function is renamed to `get_module` if you have used another name. The
macro also exports a crate-prefixed `<crate_name>_get_module` alias, used when
[statically linking the extension into php-src](../advanced/static_linking.md).
Building with `EXT_PHP_RS_STATIC_EXT=1` removes the unmangled `get_module`
export (the function itself stays callable from Rust) so a statically linked
extension cannot collide with another extension exporting `get_module`.
Dynamic builds (`extension=` / `dl()`) must not set the variable.

The function is passed an instance of `ModuleBuilder` which allows you to
register the following (if required):

- Functions, classes, and constants
- Extension and request startup and shutdown functions.
  - Read more about the PHP extension lifecycle
    [here](https://www.phpinternalsbook.com/php7/extensions_design/php_lifecycle.html).
- PHP extension information function
  - Used by the `phpinfo()` function to get information about your extension.

Classes and constants are not registered with PHP in the `get_module` function. These are
registered inside the extension startup function.

## Usage

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{
    prelude::*,
    zend::ModuleEntry,
    info_table_start,
    info_table_row,
    info_table_end
};

#[php_const]
pub const MY_CUSTOM_CONST: &'static str = "Hello, world!";

#[php_class]
pub struct Test {
    a: i32,
    b: i32
}
#[php_function]
pub fn hello_world() -> &'static str {
    "Hello, world!"
}

/// Used by the `phpinfo()` function and when you run `php -i`.
/// This will probably be simplified with another macro eventually!
pub extern "C" fn php_module_info(_module: *mut ModuleEntry) {
    info_table_start!();
    info_table_row!("my extension", "enabled");
    info_table_end!();
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .constant(wrap_constant!(MY_CUSTOM_CONST))
        .class::<Test>()
        .function(wrap_function!(hello_world))
        .info_function(php_module_info)
}
# fn main() {}
```
