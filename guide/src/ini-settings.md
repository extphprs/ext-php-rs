# INI Settings

Your PHP Extension may want to provide it's own PHP INI settings to configure behaviour. This can be done in the `#[php_startup]` annotated startup function.

## Registering INI Settings

All PHP INI definitions must be registered with PHP to get / set their values via the `php.ini` file or `ini_get() / ini_set()`.


```rust,no_run
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
# use ext_php_rs::prelude::*;
# use ext_php_rs::zend::{IniEntryDef, IniEntryDefs};
# use ext_php_rs::flags::IniEntryPermission;

static INI_ENTRIES: IniEntryDefs<2> = IniEntryDefs::new([
    IniEntryDef::new(
        c"my_extension.display_emoji",
        c"yes",
        IniEntryPermission::All,
    ),
    IniEntryDef::end(),
]);

pub fn startup(ty: i32, mod_num: i32) -> i32 {
    IniEntryDef::register(INI_ENTRIES.as_slice(), mod_num);

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

`IniEntryDef::register` takes a `&'static [IniEntryDef]` terminated by
`IniEntryDef::end()`. The engine borrows the table for the life of the process:
PHP 8.5 stores it as `zend_ini_entry.def` and the CLI SAPI dereferences
`def->value` when printing `php --ini`. `IniEntryDef::new` is a `const fn`, so
the table is a plain `static` — the same thing a C extension writes as
`zend_ini_entry_def[]` in `.rodata`.

```rust,ignore
use ext_php_rs::{flags::IniEntryPermission, zend::{IniEntryDef, IniEntryDefs}};

static INI: IniEntryDefs<2> = IniEntryDefs::new([
    IniEntryDef::new(c"my_extension.display_emoji", c"1", IniEntryPermission::All),
    IniEntryDef::end(),
]);

pub fn startup(_ty: i32, mod_num: i32) -> i32 {
    IniEntryDef::register(INI.as_slice(), mod_num);
    0
}
```
