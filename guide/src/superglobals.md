# Superglobals

PHP provides several superglobal arrays that are accessible from any scope. In
ext-php-rs, you can access these superglobals using the `ProcessGlobals`,
`SapiGlobals`, and `ExecutorGlobals` types.

> **Note for FrankenPHP users:** In FrankenPHP worker mode, you would need to
> override `sapi_activate`/`sapi_deactivate` hooks to mimic
> `php_rinit`/`php_rshutdown` behavior. However, superglobals are inaccessible
> during `sapi_activate` and will cause crashes if accessed at that point.

## Accessing HTTP Superglobals

The `ProcessGlobals` type provides access to the common HTTP superglobals:

| Method                | PHP Equivalent |
|-----------------------|----------------|
| `http_get_vars()`     | `$_GET`        |
| `http_post_vars()`    | `$_POST`       |
| `http_cookie_vars()`  | `$_COOKIE`     |
| `http_server_vars()`  | `$_SERVER`     |
| `http_env_vars()`     | `$_ENV`        |
| `http_files_vars()`   | `$_FILES`      |
| `http_request_vars()` | `$_REQUEST`    |

### Basic Example

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::ProcessGlobals;

#[php_function]
pub fn get_cookie(name: String) -> Option<String> {
    ProcessGlobals::get()
        .http_cookie_vars()
        .get(name.as_str())
        .and_then(|zval| zval.string())
}

#[php_function]
pub fn get_query_param(name: String) -> Option<String> {
    ProcessGlobals::get()
        .http_get_vars()
        .get(name.as_str())
        .and_then(|zval| zval.string())
}

#[php_function]
pub fn get_post_param(name: String) -> Option<String> {
    ProcessGlobals::get()
        .http_post_vars()
        .get(name.as_str())
        .and_then(|zval| zval.string())
}
# fn main() {}
```

### Accessing `$_SERVER`

The `$_SERVER` superglobal is lazy-initialized in PHP, so `http_server_vars()`
returns an `Option`:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::ProcessGlobals;

#[php_function]
pub fn get_request_method() -> Option<String> {
    ProcessGlobals::get()
        .http_server_vars()?
        .get("REQUEST_METHOD")
        .and_then(|zval| zval.string())
}

#[php_function]
pub fn get_remote_addr() -> Option<String> {
    ProcessGlobals::get()
        .http_server_vars()?
        .get("REMOTE_ADDR")
        .and_then(|zval| zval.string())
}

#[php_function]
pub fn get_user_agent() -> Option<String> {
    ProcessGlobals::get()
        .http_server_vars()?
        .get("HTTP_USER_AGENT")
        .and_then(|zval| zval.string())
}
# fn main() {}
```

### Working with `$_FILES`

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::ProcessGlobals;

#[php_function]
pub fn get_uploaded_file_name(field: String) -> Option<String> {
    let globals = ProcessGlobals::get();
    let files = globals.http_files_vars();

    // $_FILES structure: $_FILES['field']['name'], ['tmp_name'], ['size'], etc.
    files
        .get(field.as_str())?
        .array()?
        .get("name")
        .and_then(|zval| zval.string())
}

#[php_function]
pub fn get_uploaded_file_tmp_path(field: String) -> Option<String> {
    let globals = ProcessGlobals::get();
    let files = globals.http_files_vars();

    files
        .get(field.as_str())?
        .array()?
        .get("tmp_name")
        .and_then(|zval| zval.string())
}
# fn main() {}
```

### Returning Superglobals to PHP

You can return copies of superglobals back to PHP:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::boxed::ZBox;
use ext_php_rs::types::ZendHashTable;
use ext_php_rs::zend::ProcessGlobals;

#[php_function]
pub fn get_all_cookies() -> ZBox<ZendHashTable> {
    ProcessGlobals::get().http_cookie_vars().to_owned()
}

#[php_function]
pub fn get_all_get_params() -> ZBox<ZendHashTable> {
    ProcessGlobals::get().http_get_vars().to_owned()
}

#[php_function]
pub fn get_server_vars() -> Option<ZBox<ZendHashTable>> {
    Some(ProcessGlobals::get().http_server_vars()?.to_owned())
}
# fn main() {}
```

## SAPI Request Information

For lower-level request information, use `SapiGlobals`:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::SapiGlobals;

#[php_function]
pub fn get_request_info() -> Vec<String> {
    let globals = SapiGlobals::get();
    let request_info = globals.request_info();

    let mut info = Vec::new();

    if let Some(method) = request_info.request_method() {
        info.push(format!("Method: {}", method));
    }
    if let Some(uri) = request_info.request_uri() {
        info.push(format!("URI: {}", uri));
    }
    if let Some(query) = request_info.query_string() {
        info.push(format!("Query: {}", query));
    }
    if let Some(content_type) = request_info.content_type() {
        info.push(format!("Content-Type: {}", content_type));
    }
    info.push(format!("Content-Length: {}", request_info.content_length()));

    info
}
# fn main() {}
```

### Available Request Info Methods

| Method              | Description                   |
|---------------------|-------------------------------|
| `request_method()`  | HTTP method (GET, POST, etc.) |
| `request_uri()`     | Request URI                   |
| `query_string()`    | Query string                  |
| `cookie_data()`     | Raw cookie data               |
| `content_type()`    | Content-Type header           |
| `content_length()`  | Content-Length value          |
| `path_translated()` | Translated filesystem path    |
| `auth_user()`       | HTTP Basic auth username      |
| `auth_password()`   | HTTP Basic auth password      |

## Accessing Constants

Use `ExecutorGlobals` to access PHP constants:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::ExecutorGlobals;

#[php_function]
pub fn get_php_version_constant() -> Option<String> {
    let globals = ExecutorGlobals::get();
    globals
        .constants()?
        .get("PHP_VERSION")
        .and_then(|zval| zval.string())
}

#[php_function]
pub fn constant_exists(name: String) -> bool {
    let globals = ExecutorGlobals::get();
    globals
        .constants()
        .is_some_and(|c| c.get(name.as_str()).is_some())
}
# fn main() {}
```

## Accessing INI Values

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::ExecutorGlobals;

#[php_function]
pub fn get_memory_limit() -> Option<String> {
    ExecutorGlobals::get()
        .ini_values()
        .get("memory_limit")
        .cloned()
        .flatten()
}

#[php_function]
pub fn get_all_ini_values() -> Vec<(String, String)> {
    ExecutorGlobals::get()
        .ini_values()
        .iter()
        .filter_map(|(k, v)| {
            v.as_ref().map(|val| (k.clone(), val.clone()))
        })
        .collect()
}
# fn main() {}
```

## Thread Safety

All global access methods use guard types that provide thread-safe access:

- `ProcessGlobals::get()` returns a `GlobalReadGuard<ProcessGlobals>`
- `ExecutorGlobals::get()` returns a `GlobalReadGuard<ExecutorGlobals>`
- `SapiGlobals::get()` returns a `GlobalReadGuard<SapiGlobals>`

The guard is automatically released when it goes out of scope. For mutable
access (rarely needed), use `get_mut()`:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::zend::ExecutorGlobals;

fn example() {
    // Read-only access (most common)
    let globals = ExecutorGlobals::get();
    // ... use globals ...
    // Guard released when `globals` goes out of scope

    // Mutable access (rarely needed)
    let mut globals = ExecutorGlobals::get_mut();
    // ... modify globals ...
}
# fn main() {}
```

## PHP Example

```php
<?php

// Assuming you've registered the functions above

// Access cookies
$session_id = get_cookie('session_id');

// Access query parameters
$page = get_query_param('page') ?? '1';

// Get request method
$method = get_request_method(); // "GET", "POST", etc.

// Get all cookies as array
$all_cookies = get_all_cookies();
print_r($all_cookies);

// Get request info
$info = get_request_info();
print_r($info);

// Access constants
$php_version = get_php_version_constant();
echo "PHP Version: $php_version\n";

// Check if constant exists
if (constant_exists('MY_CUSTOM_CONSTANT')) {
    echo "Constant exists!\n";
}
```

## Summary

| Type              | Use Case                                                |
|-------------------|---------------------------------------------------------|
| `ProcessGlobals`  | HTTP superglobals (`$_GET`, `$_POST`, `$_COOKIE`, etc.) |
| `SapiGlobals`     | Low-level request info, headers                         |
| `ExecutorGlobals` | Constants, INI values, function/class tables            |

All types are accessed via `::get()` for read access or `::get_mut()` for write
access, and provide thread-safe access through guard types.
