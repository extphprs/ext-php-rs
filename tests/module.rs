//! Module tests
#![cfg_attr(windows, feature(abi_vectorcall))]
#![cfg(feature = "embed")]
#![allow(
    missing_docs,
    clippy::needless_pass_by_value,
    clippy::must_use_candidate
)]
extern crate ext_php_rs;

use cfg_if::cfg_if;

use ext_php_rs::embed::Embed;
use ext_php_rs::ffi::zend_register_module_ex;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::ExecutorGlobals;

#[test]
fn test_module() {
    Embed::run(|| {
        // Allow to load the module
        cfg_if! {
            if #[cfg(php84)] {
                // Register as temporary (2) module
                unsafe { zend_register_module_ex(get_module(), 2) };
                // When registering temporary modules directly (bypassing dl()),
                // we must set full_tables_cleanup to ensure proper cleanup of
                // request-scoped interned strings used as module registry keys.
                // Without this, the interned string is freed during
                // zend_interned_strings_deactivate() while module_registry still
                // references it, causing heap corruption.
                ExecutorGlobals::get_mut().full_tables_cleanup = true;
            } else {
                unsafe { zend_register_module_ex(get_module()) };
            }
        }

        let result = Embed::eval("$foo = hello_world('foo');");

        assert!(result.is_ok());

        let zval = result.unwrap();

        assert!(zval.is_string());

        let string = zval.string().unwrap();

        assert_eq!(string.clone(), "Hello, foo!");

        // The engine keeps `arg_info[i].name` and `.default_value` by pointer for
        // the life of the process, so reflecting over a registered function reads
        // the tables `StaticModuleEntry` owns. Under LSan this is what proves they
        // are still reachable rather than lost.
        let reflected = Embed::eval(
            "$out = (function () {
                 $r = new ReflectionFunction('greet');
                 $p = $r->getParameters()[0];
                 return $p->getName() . '|' . $p->getDefaultValue() . '|' . $r->getReturnType();
             })();",
        );

        assert!(reflected.is_ok());
        assert_eq!(
            reflected.unwrap().string().unwrap().clone(),
            "name|world|string"
        );
    });
}

/// Gives you a nice greeting!
///
/// @param string $name Your name.
///
/// @return string Nice greeting!
#[php_function]
pub fn hello_world(name: String) -> String {
    format!("Hello, {name}!")
}

/// Registers an argument carrying a name, a default value and a declared type,
/// plus a declared return type, so the `ModuleAllocations` arg-info table has
/// something to hold.
#[php_function]
#[php(defaults(name = "world".to_string()))]
pub fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .function(wrap_function!(hello_world))
        .function(wrap_function!(greet))
}
