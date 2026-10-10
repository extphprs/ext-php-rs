# Worker Mode

Worker mode lets you recycle the PHP engine between requests without the
overhead of a full `php_request_shutdown()` / `php_request_startup()` cycle.
This is useful for long-running servers that serve many requests on the same
thread.

## How it works

A full PHP request cycle destroys and re-creates the executor state. Worker
mode instead performs a lightweight shutdown that tears down only the SAPI
and output layers, then re-activates them for the next request. This keeps
compiled classes and functions in memory while resetting request-scoped state.

## API

```rust,ignore
use ext_php_rs::embed::{
    worker_request_shutdown,
    worker_request_startup,
    worker_reset_superglobals,
};

// After processing a request:
worker_request_shutdown();

// Before the next request:
worker_request_startup().expect("startup failed");
worker_reset_superglobals();
```

## Typical lifecycle

```text
1. sapi_startup() + php_module_startup()    -- once per process
2. php_request_startup()                    -- first request
3. execute PHP script
4. worker_request_shutdown()                -- lightweight teardown
5. worker_request_startup()                 -- lightweight re-init
6. worker_reset_superglobals()              -- refresh $_SERVER etc.
7. execute PHP script                       -- next request
   ... repeat 4-7 ...
8. php_request_shutdown()                   -- final cleanup
9. php_module_shutdown() + sapi_shutdown()  -- once per process
```

## ZTS and `PhpThreadGuard`

When PHP is compiled with ZTS (Zend Thread Safety), each OS thread needs its
own thread-local storage. Use `PhpThreadGuard` to manage this automatically:

```rust,ignore
use ext_php_rs::embed::PhpThreadGuard;

std::thread::spawn(|| {
    let _guard = PhpThreadGuard::new();
    // This thread can now run PHP requests.
    // TLS is cleaned up when _guard is dropped.
});
```

The guard must be dropped before `php_module_shutdown()` is called.

## Combining with the Sapi trait

Worker mode pairs naturally with a custom `Sapi` implementation. Build the
SAPI module once, start it, then use worker mode to cycle between requests
without tearing down the full engine.

## Run code at the start of each request

In worker mode, PHP does not call the request startup function of an
extension for each request. This is also true for a `FrankenPHP` worker. To
run code for each request, use `ModuleBuilder::sapi_activate_function`.

```rust,ignore
use ext_php_rs::prelude::*;
use ext_php_rs::zend::{SapiRequestInfo, set_header, set_response_code};

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.sapi_activate_function(|info: &SapiRequestInfo| {
        if info.request_uri() == Some("/old") {
            let _ = set_header("Location: /new");
            let _ = set_response_code(301);
        }
    })
}
```

PHP calls the closure from `sapi_module.activate` at the start of each
request. This includes each request of a `FrankenPHP` worker, and the first
request that starts the worker script.

The closure gets the request data from the SAPI: the method, the URI, the
query string and the cookies. The response headers are empty, so
`set_header` and `set_response_code` work. `$_SERVER` does not exist yet, and
PHP code cannot run.

Obey these rules:

- Do not load the extension with `dl()`. PHP removes the extension at the end
  of the request, but the SAPI keeps the hook. The extension refuses to start.
- Do not use a panic for control. PHP prints the panic message and continues
  the request.

A redirect in the closure does not stop the application. The script, or the
worker callback, runs after the closure. It can replace the headers and write
a body.
