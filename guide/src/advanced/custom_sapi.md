# Custom SAPI

ext-php-rs allows you to implement a fully custom SAPI (Server API) in Rust
using the `Sapi` trait. This lets you embed the PHP engine inside your own
server, such as an HTTP framework, a message queue consumer, or any other
application that needs to drive PHP request processing.

## Overview

A SAPI implementation consists of three parts:

1. **`ServerContext`** -- per-request state (method, URI, POST body, headers).
2. **`Sapi`** -- the SAPI itself: output, logging, header sending.
3. **Trampolines** -- generated automatically by `Sapi::build_module()`.

## Defining a `ServerContext`

Implement the `ServerContext` trait for your per-request type:

```rust,ignore
use ext_php_rs::embed::{ServerContext, RequestInfo};

struct MyContext {
    method: String,
    uri: String,
    body: Vec<u8>,
    body_offset: usize,
    finished: bool,
}

impl ServerContext for MyContext {
    fn init_request_info(&self, info: &mut RequestInfo) {
        info.request_method = Some(self.method.clone());
        info.request_uri = Some(self.uri.clone());
        info.content_length = self.body.len() as i64;
    }

    fn read_post(&mut self, buf: &mut [u8]) -> usize {
        let remaining = &self.body[self.body_offset..];
        let n = buf.len().min(remaining.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.body_offset += n;
        n
    }

    fn read_cookies(&self) -> Option<&str> {
        None
    }

    fn finish_request(&mut self) -> bool {
        if self.finished {
            return false;
        }
        self.finished = true;
        true
    }

    fn is_request_finished(&self) -> bool {
        self.finished
    }
}
```

## Implementing the `Sapi` trait

```rust,ignore
use ext_php_rs::embed::{Sapi, ServerContext, RequestInfo, ServerVarRegistrar};

# struct MyContext { method: String, uri: String, body: Vec<u8>, body_offset: usize, finished: bool }
# impl ServerContext for MyContext {
#     fn init_request_info(&self, _: &mut RequestInfo) {}
#     fn read_post(&mut self, _: &mut [u8]) -> usize { 0 }
#     fn read_cookies(&self) -> Option<&str> { None }
#     fn finish_request(&mut self) -> bool { true }
#     fn is_request_finished(&self) -> bool { true }
# }
struct MySapi;

impl Sapi for MySapi {
    type Context = MyContext;

    fn name() -> &'static str { "my-sapi" }
    fn pretty_name() -> &'static str { "My Custom SAPI" }

    fn ub_write(_ctx: &mut MyContext, buf: &[u8]) -> usize {
        // Forward output to your HTTP response
        print!("{}", String::from_utf8_lossy(buf));
        buf.len()
    }

    fn log_message(msg: &str, _syslog_type: i32) {
        eprintln!("[php] {msg}");
    }
}
```

## Building and starting the SAPI

```rust,ignore
use ext_php_rs::embed::Sapi;
# struct MySapi;
# impl Sapi for MySapi {
#     type Context = ();
#     fn name() -> &'static str { "x" }
#     fn pretty_name() -> &'static str { "x" }
#     fn ub_write(_ctx: &mut (), _buf: &[u8]) -> usize { 0 }
#     fn log_message(_: &str, _: i32) {}
# }
# impl ext_php_rs::embed::ServerContext for () {
#     fn init_request_info(&self, _: &mut ext_php_rs::embed::RequestInfo) {}
#     fn read_post(&mut self, _: &mut [u8]) -> usize { 0 }
#     fn read_cookies(&self) -> Option<&str> { None }
#     fn finish_request(&mut self) -> bool { true }
#     fn is_request_finished(&self) -> bool { true }
# }

let module = MySapi::build_module().expect("failed to build SAPI module");
```

The returned `SapiModule` can then be passed to `sapi_startup()` and
`php_module_startup()` just like a manually-built one.

## Registering `$_SERVER` variables

Override `register_server_variables` in the `Sapi` trait to populate
`$_SERVER`:

```rust,ignore
use ext_php_rs::embed::{Sapi, ServerVarRegistrar};

# struct MySapi;
# struct Ctx;
# impl ext_php_rs::embed::ServerContext for Ctx {
#     fn init_request_info(&self, _: &mut ext_php_rs::embed::RequestInfo) {}
#     fn read_post(&mut self, _: &mut [u8]) -> usize { 0 }
#     fn read_cookies(&self) -> Option<&str> { None }
#     fn finish_request(&mut self) -> bool { true }
#     fn is_request_finished(&self) -> bool { true }
# }
# impl Sapi for MySapi {
#     type Context = Ctx;
#     fn name() -> &'static str { "x" }
#     fn pretty_name() -> &'static str { "x" }
#     fn ub_write(_: &mut Ctx, b: &[u8]) -> usize { b.len() }
#     fn log_message(_: &str, _: i32) {}
    fn register_server_variables(
        _ctx: &mut Ctx,
        registrar: &mut ServerVarRegistrar,
    ) {
        registrar.register("SERVER_SOFTWARE", "my-server/1.0");
        registrar.register("SERVER_PROTOCOL", "HTTP/1.1");
    }
# }
```
