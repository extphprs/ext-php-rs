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
