use ext_php_rs::{call_user_func_named, prelude::*, types::Zval};

#[php_function]
pub fn test_callable(call: ZendCallable, a: String) -> Zval {
    call.try_call(vec![&a]).expect("Failed to call function")
}

/// Test calling a callable with only named arguments
#[php_function]
pub fn test_callable_named(call: ZendCallable) -> Zval {
    call.try_call_named(&[("b", &"second"), ("a", &"first")])
        .expect("Failed to call function with named args")
}

/// Test calling a callable with positional + named arguments
#[php_function]
pub fn test_callable_mixed(call: ZendCallable) -> Zval {
    call.try_call_with_named(&[&"positional"], &[("named", &"named_value")])
        .expect("Failed to call function with mixed args")
}

/// Test the `call_user_func_named!` macro with named arguments only
#[php_function]
pub fn test_callable_macro_named(call: ZendCallable) -> Zval {
    call_user_func_named!(call, x: "hello", y: "world").expect("Failed to call function via macro")
}

/// Test the `call_user_func_named!` macro with positional + named arguments
#[php_function]
pub fn test_callable_macro_mixed(call: ZendCallable) -> Zval {
    call_user_func_named!(call, ["first"], second: "second_val")
        .expect("Failed to call function via macro with mixed args")
}

/// Test calling with empty named params (should behave like `try_call`)
#[php_function]
pub fn test_callable_empty_named(call: ZendCallable) -> Zval {
    call.try_call_with_named(&[&"hello"], &[])
        .expect("Failed to call function with empty named args")
}

/// Test calling a built-in PHP function with named arguments
#[php_function]
pub fn test_callable_builtin_named() -> Zval {
    let str_replace =
        ZendCallable::try_from_name("str_replace").expect("Failed to get str_replace");
    str_replace
        .try_call_named(&[
            ("subject", &"Hello world"),
            ("replace", &"PHP"),
            ("search", &"world"),
        ])
        .expect("Failed to call str_replace with named args")
}

/// Test calling with duplicate named params
#[php_function]
pub fn test_callable_duplicate_named(call: ZendCallable) -> Zval {
    // When duplicates are passed, the last value wins (PHP hash table behavior)
    call.try_call_named(&[("a", &"first"), ("a", &"overwritten")])
        .expect("Failed to call function with duplicate named args")
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_callable))
        .function(wrap_function!(test_callable_named))
        .function(wrap_function!(test_callable_mixed))
        .function(wrap_function!(test_callable_macro_named))
        .function(wrap_function!(test_callable_macro_mixed))
        .function(wrap_function!(test_callable_empty_named))
        .function(wrap_function!(test_callable_builtin_named))
        .function(wrap_function!(test_callable_duplicate_named))
}

#[cfg(test)]
mod tests {
    #[test]
    fn callable_works() {
        assert!(crate::integration::test::run_php("callable/callable.php"));
    }
}
