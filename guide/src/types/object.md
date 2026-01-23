# Object

An object is any object type in PHP. This can include a PHP class and PHP
`stdClass`. A Rust struct registered as a PHP class is a [class object], which
contains an object.

Objects are valid as parameters but only as an immutable or mutable reference.
You cannot take ownership of an object as objects are reference counted, and
multiple zvals can point to the same object. You can return a boxed owned
object.

| `T` parameter | `&T` parameter | `T` Return type    | `&T` Return type  | PHP representation |
| ------------- | -------------- | ------------------ | ----------------- | ------------------ |
| No            | Yes            | `ZBox<ZendObject>` | Yes, mutable only | Zend object.       |

## Examples

### Calling a method

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{prelude::*, types::ZendObject};

// Take an object reference and also return it.
#[php_function]
pub fn take_obj(obj: &mut ZendObject) -> () {
    let _ = obj.try_call_method("hello", vec![&"arg1", &"arg2"]);
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(take_obj))
}
# fn main() {}
```

### Taking an object reference

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{prelude::*, types::ZendObject};

// Take an object reference and also return it.
#[php_function]
pub fn take_obj(obj: &mut ZendObject) -> &mut ZendObject {
    let _ = obj.set_property("hello", 5);
    dbg!(obj)
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(take_obj))
}
# fn main() {}
```

### Creating a new object

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{prelude::*, types::ZendObject, boxed::ZBox};

// Create a new `stdClass` and return it.
#[php_function]
pub fn make_object() -> ZBox<ZendObject> {
    let mut obj = ZendObject::new_stdclass();
    let _ = obj.set_property("hello", 5);
    obj
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(make_object))
}
# fn main() {}
```

## Lazy Objects (PHP 8.4+)

PHP 8.4 introduced lazy objects, which defer their initialization until their
properties are first accessed. ext-php-rs provides APIs to introspect and create
lazy objects from Rust.

### Lazy Object Types

There are two types of lazy objects:

- **Lazy Ghosts**: The ghost object itself becomes the real instance when
  initialized. After initialization, the ghost is indistinguishable from a
  regular object.

- **Lazy Proxies**: A proxy wraps a real instance that is created when first
  accessed. The proxy and real instance have different identities. After
  initialization, the proxy still reports as lazy.

### Introspection APIs

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{prelude::*, types::ZendObject};

#[php_function]
pub fn check_lazy(obj: &ZendObject) -> String {
    if obj.is_lazy() {
        if obj.is_lazy_ghost() {
            if obj.is_lazy_initialized() {
                "Initialized lazy ghost".into()
            } else {
                "Uninitialized lazy ghost".into()
            }
        } else if obj.is_lazy_proxy() {
            if obj.is_lazy_initialized() {
                "Initialized lazy proxy".into()
            } else {
                "Uninitialized lazy proxy".into()
            }
        } else {
            "Unknown lazy type".into()
        }
    } else {
        "Not a lazy object".into()
    }
}
# fn main() {}
```

Available introspection methods:

| Method | Description |
|--------|-------------|
| `is_lazy()` | Returns `true` if the object is lazy (ghost or proxy) |
| `is_lazy_ghost()` | Returns `true` if the object is a lazy ghost |
| `is_lazy_proxy()` | Returns `true` if the object is a lazy proxy |
| `is_lazy_initialized()` | Returns `true` if the lazy object has been initialized |
| `lazy_init()` | Triggers initialization of a lazy object |
| `lazy_get_instance()` | For proxies, returns `Option<&mut Self>` with the real instance after initialization |

You can also check if a class supports lazy objects using `ClassEntry::can_be_lazy()`:

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{prelude::*, zend::ClassEntry};

#[php_function]
pub fn can_class_be_lazy(class_name: &str) -> bool {
    ClassEntry::try_find(class_name)
        .map(|ce| ce.can_be_lazy())
        .unwrap_or(false)
}
# fn main() {}
```

### Creating Lazy Objects from Rust

You can create lazy objects from Rust using `make_lazy_ghost()` and
`make_lazy_proxy()`. These methods require the `closure` feature:

```toml
[dependencies]
ext-php-rs = { version = "0.15", features = ["closure"] }
```

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{prelude::*, types::ZendObject, boxed::ZBox};

#[php_function]
pub fn create_lazy_ghost(obj: &mut ZendObject) -> PhpResult<()> {
    let init_value = "initialized".to_string();
    obj.make_lazy_ghost(Box::new(move || {
        // Initialization logic - use captured state
        println!("Initializing with: {}", init_value);
    }) as Box<dyn Fn()>)?;
    Ok(())
}

#[php_function]
pub fn create_lazy_proxy(obj: &mut ZendObject) -> PhpResult<()> {
    obj.make_lazy_proxy(Box::new(|| {
        // Return the real instance
        Some(ZendObject::new_stdclass())
    }) as Box<dyn Fn() -> Option<ZBox<ZendObject>>>)?;
    Ok(())
}
# fn main() {}
```

### Creating Lazy Objects from PHP

For full control over lazy object creation, use PHP's Reflection API:

```php
<?php
// Create a lazy ghost
$reflector = new ReflectionClass(MyClass::class);
$ghost = $reflector->newLazyGhost(function ($obj) {
    $obj->__construct('initialized');
});

// Create a lazy proxy
$proxy = $reflector->newLazyProxy(function ($obj) {
    return new MyClass('initialized');
});
```

### Limitations

- **PHP 8.4+ only**: Lazy objects are a PHP 8.4 feature and not available in
  earlier versions.

- **`closure` feature required**: The `make_lazy_ghost()` and `make_lazy_proxy()`
  methods require the `closure` feature to be enabled.

- **User-defined classes only**: PHP lazy objects only work with user-defined
  PHP classes, not internal classes. Since Rust-defined classes (using
  `#[php_class]`) are registered as internal classes, they cannot be made lazy.

- **Closure parameter access**: Due to Rust trait system limitations, the
  `make_lazy_ghost()` and `make_lazy_proxy()` closures don't receive the object
  being initialized as a parameter. Capture any needed initialization state in
  the closure itself.

[class object]: ./class_object.md
