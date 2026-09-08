# Memory Model at MINIT

Registering classes, functions, INI entries and constants builds C tables that
the Zend engine reads. A C extension puts those tables in `.rodata`, so the
linker owns them for the whole process and the question never comes up.
ext-php-rs builds them on the heap at MINIT, so ownership has to be decided.

Each table falls into one of two buckets. Nothing sits in between, which is why
ext-php-rs has no shutdown cleanup hook.

## Reclaimed as soon as the engine has consumed them

Class and enum method tables. `zend_register_functions` interns every function
name into a `zend_string`, and `do_register_internal_class` reads
`zend_class_entry.info.internal.builtin_functions` exactly once and never frees
it. The table is dead the moment registration returns, and ext-php-rs frees it
there.

## Owned for the life of the process

Everything the engine keeps a pointer into:

- **Argument info**, including argument names, default values and class names in
  types. `zend_register_functions` copies the array only when the types it
  carries make the engine derive `ZEND_ACC_HAS_RETURN_TYPE` or
  `ZEND_ACC_HAS_TYPE_HINTS`, and that copy is shallow: `name` and
  `default_value` stay borrowed and are read at runtime by
  `ReflectionParameter`. A function with neither parameters nor a return type
  keeps the original array.
- **The module function table**, its function names, and the module `name` and
  `version`. `module_destructor` walks the table reading `fname` after
  MSHUTDOWN for `dl()`-loaded extensions, `get_extension_funcs()` walks it
  during a request, and a second `php_module_startup` in the same process is
  handed the same static entry.
- **The INI definition table** and its strings. PHP 8.5 stores it as
  `zend_ini_entry.def` and the CLI SAPI reads `def->value` for
  `php --ini=diff`.

ext-php-rs keeps these reachable from a crate-level list rather than orphaning
them. That costs one lock per table at MINIT, nothing per request, and it is
what makes a leak detector report them as still-reachable instead of lost.

## What this means for extension authors

Do not free anything you hand to a `zend_register_*` function, and do not free a
`ModuleEntry` field. There is no cleanup entry point to call: `#[php_module]`
leaves `module_shutdown_func` exactly as you set it.

Class constants and enum case discriminants are the one place to be careful when
building tables by hand. `zend_declare_class_constant` and `zend_enum_add_case`
take the zval payload with `ZVAL_COPY_VALUE` and no reference bump, so the
engine owns any refcounted value afterwards and the Rust `Zval` must not run its
destructor.
