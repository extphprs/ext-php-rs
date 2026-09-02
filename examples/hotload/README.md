# ext-php-rs-hotload: Hot-load Rust into PHP

Load Rust code into PHP at runtime without rebuilding PHP or restarting the server.

## Quick Start

```bash
# Build the extension (from ext-php-rs root)
cargo build --release -p ext-php-rs-hotload

# Run the demo
cd examples/hotload
php -dextension="../../target/release/libext_php_rs_hotload.dylib" demo_host.php
```

## Writing Rust Code

Just write `.rs` files with your functions:

```rust
//! Hello module

#[php_function]
fn hello(name: String) -> String {
    format!("Hello, {}!", name)
}

#[php_function]
fn add(a: i64, b: i64) -> i64 {
    a + b
}
```

That's it! The extension automatically:
- Adds `ext-php-rs` dependency
- Adds `use ext_php_rs::prelude::*`
- Generates `#[php_module]` from `#[php_function]` declarations
- Handles compilation and caching

## PHP Usage

### Loading from Files (Global Functions)

```php
// Load from file (compiles on first load, cached after)
RustHotload::loadFile('hello.rs');

// Functions are now available globally
echo hello("World");  // "Hello, World!"
echo add(2, 3);       // 5
```

### Loading from Strings (Global Functions)

Use `loadString()` to load Rust code from a string with global function registration:

```php
RustHotload::loadString('
    #[php_function]
    fn square(x: i64) -> i64 { x * x }

    #[php_function]
    fn cube(x: i64) -> i64 { x * x * x }
');

echo square(5);  // 25
echo cube(5);    // 125
```

### Loading from Cargo Projects

Use `loadDir()` to load a full Cargo project directory:

```php
// Load a Cargo project (builds in-place, rebuilds on source changes)
RustHotload::loadDir('/path/to/my-extension');

// Functions from the project are now available
echo my_function();
```

The project must be a valid Cargo workspace with:
- `Cargo.toml` with `crate-type = ["cdylib"]`
- Functions registered via `#[php_function]` and `wrap_function!`
- The hotload ABI exports (`hotload_info`, `hotload_init`)

See `examples/hotload/my_module/` for a complete example.

### Single Callable Functions

Use `func()` to create a single callable function:

```php
$triple = RustHotload::func('(x: i64) -> i64', 'x * 3');
$add = RustHotload::func('(a: i64, b: i64) -> i64', 'a + b');

echo $triple(14);   // 42
echo $add(10, 32);  // 42

// Pass to array_map, usort, etc.
$numbers = [1, 2, 3, 4, 5];
$tripled = array_map($triple, $numbers);  // [3, 6, 9, 12, 15]

// With external dependencies (use statements at start of body)
$to_json = RustHotload::func('(data: Vec<i64>) -> String', '
    use serde_json; // 1.0
    serde_json::to_string(&data).unwrap_or_default()
');
```

### Rust Classes with State

Use `class()` to create Rust structs with fields and methods. Add `use` statements
at the start of the methods string to include external dependencies:

```php
$Counter = RustHotload::class(
    'value: i64',                                       // struct fields
    '
    __construct(start: i64) { Self { value: start } }   // constructor
    add(&mut self, b: i64) { self.value += b }          // mutable method
    get(&self) -> i64 { self.value }                    // immutable method
    '
);

$c = $Counter(5);        // Create instance with initial value 5
echo $c->get();          // 5
$c->add(10);
echo $c->get();          // 15
```

### Utility Functions

```php
// List loaded modules
print_r(RustHotload::list());

// Cache management
echo RustHotload::cacheDir();
RustHotload::clearCache();

// Manually unload a module by name
RustHotload::unload("module_name");

// Debug mode (release by default)
RustHotload::setDebug(true);   // Enable debug builds
RustHotload::setDebug(false);  // Back to release builds
echo RustHotload::isDebug();   // Check current mode
```

## SAPI Compatibility (PHP-FPM, Apache)

**Global functions** (from `loadFile()`, `loadDir()`, and `loadString()`):
- Unregistered at end of each request
- Modules stay cached for fast re-registration on next request
- No function name conflicts between requests

**Scoped functions** (from `func()` and `class()`):
- Use prefixed internal names, managed by wrapper objects
- Stay registered while the PHP object exists

**Concurrent safety**: When multiple processes/threads operate simultaneously:
- **Compilation**: File locking ensures only one process compiles while others wait
- **Loading**: Module loading is serialized to prevent duplicate registrations
- **Module cache**: Protected by mutex for thread-safe access

Set `HOTLOAD_VERBOSE=1` to trace operations:
```bash
HOTLOAD_VERBOSE=1 php script.php
# Output: [hotload] Module 'math' re-registering functions
#         [hotload] Request shutdown: unregistering 4 global functions
#         [hotload] Request complete (2 modules cached)
```

## Adding Dependencies

Dependencies are auto-detected from `use` statements. Add version in a comment:

```rust
use serde::{Deserialize, Serialize}; // 1.0, features = ["derive"]
use serde_json; // 1.0

#[php_function]
fn my_function() -> String {
    // serde and serde_json are available
}
```

For crates with hyphenated names (like `tree-sitter`), use `as` to specify the package name:

```rust
use tree_sitter::Parser; // 0.23 as tree-sitter
```

Version comment formats:
- `// 1.0` - version only
- `// 1.0, features = ["derive"]` - with features
- `// 0.23 as tree-sitter` - explicit package name
- `// 1.0, features = ["derive"] as my-crate` - all options
- `// path = "../my-crate"` - local path dependency
- `// git = "https://github.com/user/repo"` - git dependency
- `// git = "https://github.com/user/repo", branch = "main"` - git with branch
- `// git = "https://github.com/user/repo", tag = "v1.0"` - git with tag
