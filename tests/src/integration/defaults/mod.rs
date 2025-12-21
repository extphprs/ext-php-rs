use ext_php_rs::prelude::*;

#[php_function]
#[php(defaults(a = 42))]
pub fn test_defaults_integer(a: i32) -> i32 {
    a
}

#[php_function]
#[php(defaults(a = None))]
pub fn test_defaults_nullable_string(a: Option<String>) -> Option<String> {
    a
}

#[allow(clippy::unnecessary_wraps)]
#[php_function]
#[php(defaults(a = None, b = None))]
pub fn test_defaults_multiple_option_arguments(
    a: Option<String>,
    b: Option<String>,
) -> PhpResult<String> {
    Ok(a.or(b).unwrap_or_else(|| "Default".to_string()))
}

#[php_function]
#[php(defaults(a = Some("fallback".to_string())))]
pub fn test_defaults_nullable_with_some_default(a: Option<String>) -> Option<String> {
    a
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_defaults_integer))
        .function(wrap_function!(test_defaults_nullable_string))
        .function(wrap_function!(test_defaults_multiple_option_arguments))
        .function(wrap_function!(test_defaults_nullable_with_some_default))
}

#[cfg(test)]
mod tests {
    #[test]
    fn defaults_works() {
        assert!(crate::integration::test::run_php("defaults/defaults.php"));
    }
}
