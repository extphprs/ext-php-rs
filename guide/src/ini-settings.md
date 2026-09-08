# INI Settings

Your PHP Extension may want to provide it's own PHP INI settings to configure behaviour. This can be done in the `#[php_startup]` annotated startup function.

## Registering INI Settings

All PHP INI definitions must be registered with PHP to get / set their values via the `php.ini` file or `ini_get() / ini_set()`.


```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
# use ext_php_rs::zend::IniEntryDef;
# use ext_php_rs::flags::IniEntryPermission;

pub fn startup(ty: i32, mod_num: i32) -> i32 {
    let ini_entries: Vec<IniEntryDef> = vec![
        IniEntryDef::new(
            "my_extension.display_emoji".to_owned(),
            "yes".to_owned(),
            &IniEntryPermission::All,
        ),
    ];
    IniEntryDef::register(ini_entries, mod_num);

    0
}

#[php_module]
#[php(startup = "startup")]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
}
# fn main() {}
```

## Getting INI Settings

The INI values are stored as part of the `GlobalExecutor`, and can be accessed via the `ini_values()` function. To retrieve the value for a registered INI setting

```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::{
    prelude::*,
    zend::ExecutorGlobals,
};

pub fn startup(ty: i32, mod_num: i32) -> i32 {
    // Get all INI values
    let ini_values = ExecutorGlobals::get().ini_values(); // HashMap<String, Option<String>>
    let my_ini_value = ini_values.get("my_extension.display_emoji"); // Option<Option<String>>

    0
}

#[php_module]
#[php(startup = "startup")]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
}
# fn main() {}
```

## Memory

`IniEntryDef::register` keeps its definition table for the life of the process.
PHP 8.5 stores it as `zend_ini_entry.def` and the CLI SAPI dereferences
`def->value` when printing `php --ini=diff`, so the table cannot be freed after
registration. This is the same lifetime a C extension gets for free by putting
its `zend_ini_entry_def[]` in `.rodata`.
