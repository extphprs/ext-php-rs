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

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_callable))
        .function(wrap_function!(test_callable_named))
        .function(wrap_function!(test_callable_mixed))
        .function(wrap_function!(test_callable_macro_named))
        .function(wrap_function!(test_callable_macro_mixed))
}

#[cfg(test)]
mod tests {
    #[test]
    fn callable_works() {
        assert!(crate::integration::test::run_php("callable/callable.php"));
    }
}
