use ext_php_rs::{
    prelude::*,
    types::{PhpRef, ZendHashTable},
};

type MaybeAge = Option<i64>;
type Ref<'a> = PhpRef<'a>;

#[php_class]
pub struct AliasCounter {
    #[php(prop)]
    count: i64,
}

#[php_impl]
impl AliasCounter {
    pub fn __construct() -> Self {
        Self { count: 0 }
    }
}

#[php_function]
pub fn test_alias_option(age: MaybeAge) -> i64 {
    age.unwrap_or(-1)
}

#[php_function]
pub fn test_alias_nullable_required(a: MaybeAge, b: i64) -> i64 {
    a.unwrap_or(0) + b
}

#[php_function]
pub fn test_qualified_option(name: std::option::Option<String>) -> String {
    name.unwrap_or_else(|| "nobody".to_string())
}

#[php_function]
pub fn test_qualified_variadic(items: &[&ext_php_rs::types::Zval]) -> usize {
    items.len()
}

#[php_function]
pub fn test_typed_variadic(numbers: &[i64]) -> i64 {
    numbers.iter().sum()
}

#[php_function]
pub fn test_alias_phpref(mut target: Ref<'_>) {
    if let Some(n) = target.long() {
        target.set_long(n + 1);
    }
}

#[php_function]
pub fn test_object_by_value(counter: &mut AliasCounter) -> i64 {
    counter.count += 1;
    counter.count
}

#[php_function]
pub fn test_array_by_ref(arr: &mut ZendHashTable) {
    let _ = arr.push(1);
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .class::<AliasCounter>()
        .function(wrap_function!(test_alias_option))
        .function(wrap_function!(test_alias_nullable_required))
        .function(wrap_function!(test_qualified_option))
        .function(wrap_function!(test_qualified_variadic))
        .function(wrap_function!(test_typed_variadic))
        .function(wrap_function!(test_alias_phpref))
        .function(wrap_function!(test_object_by_value))
        .function(wrap_function!(test_array_by_ref))
}

#[cfg(test)]
mod tests {
    #[test]
    fn aliases_work() {
        assert!(crate::integration::test::run_php("aliases/aliases.php"));
    }
}
