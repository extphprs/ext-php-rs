use ext_php_rs::prelude::*;
use ext_php_rs::types::ZendClassObject;

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

#[php_function]
#[php(defaults(greeting = "\"hi\""))]
pub fn test_defaults_str(greeting: &str) -> String {
    greeting.to_uppercase()
}

#[php_function]
#[php(defaults(ratio = 1.5))]
pub fn test_defaults_float(ratio: f64) -> f64 {
    ratio * 2.0
}

#[php_class]
pub struct OptionalArgs {
    prefix: String,
}

#[php_impl]
impl OptionalArgs {
    #[php(optional = suffix)]
    pub fn __construct(prefix: String, suffix: Option<String>) -> Self {
        Self {
            prefix: format!("{prefix}{}", suffix.unwrap_or_default()),
        }
    }

    #[php(optional = count, defaults(times = 2))]
    pub fn repeat(&self, count: Option<i64>, times: i64) -> String {
        let total = count.unwrap_or(1) * times;
        self.prefix.repeat(usize::try_from(total).unwrap_or(0))
    }

    #[php(optional = second)]
    pub fn pick(
        self_: &mut ZendClassObject<OptionalArgs>,
        first: Option<i64>,
        second: Option<i64>,
    ) -> String {
        format!("{}{:?}{:?}", self_.prefix, first, second)
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .class::<OptionalArgs>()
        .function(wrap_function!(test_defaults_str))
        .function(wrap_function!(test_defaults_float))
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
