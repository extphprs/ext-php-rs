# INI Builder

When configuring a SAPI you may use `IniBuilder` to load INI settings as text.
This is useful for setting up configurations required by the SAPI capabilities.

INI settings applied to a SAPI through `sapi.ini_entries` will be immutable,
meaning they cannot be changed at runtime. This is useful for applying settings
to match hard requirements of the way your SAPI works.

To apply _configurable_ defaults it is recommended to use a `sapi.ini_defaults`
callback instead, which will allow settings to be changed at runtime.

```rust,no_run,ignore
use ext_php_rs::builder::{IniBuilder, SapiBuilder};

# fn main() {
// Create a new IniBuilder instance.
let mut builder = IniBuilder::new();

// Append a single key/value pair to the INIT buffer with an unquoted value.
builder.unquoted("log_errors", "1");

// Append a single key/value pair to the INI buffer with a quoted value.
builder.quoted("default_mimetype", "text/html");

// Append INI line text as-is. A line break will be automatically appended.
builder.define("memory_limit=128MB");

// Prepend INI line text as-is. No line break insertion will occur.
builder.prepend("error_reporting=0\ndisplay_errors=1\n");

// Construct and start the SAPI.
let mut sapi = SapiBuilder::new("name", "pretty_name").build()
  .expect("should build SAPI");
unsafe { sapi_startup(&raw mut sapi) };

// Hand the INI entries to PHP before the engine starts.
sapi.ini_entries = builder.finish().as_ptr();
unsafe { php_module_startup(&raw mut sapi, get_module()) };
# }
```

`sapi_startup` clears `ini_entries` before copying the module, so the
assignment has to happen between `sapi_startup` and `php_module_startup`, which
is where php-src's own SAPIs set it. `SapiBuilder::ini_entries` is discarded by
that clear for the same reason. The buffer stays owned by the `IniBuilder`: keep
the builder alive until `php_module_startup` has returned, and do not expect
`cleanup_sapi_allocations` to free it.
