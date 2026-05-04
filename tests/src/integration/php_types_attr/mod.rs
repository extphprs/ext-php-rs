use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

#[php_class]
pub struct PhpTypesAttrFoo;

#[php_class]
pub struct PhpTypesAttrBar;

#[php_class]
pub struct PhpTypesAttrHolder;

#[php_impl]
impl PhpTypesAttrHolder {
    pub fn __construct() -> Self {
        Self
    }

    pub fn accept(#[php(types = "int|string")] _value: &Zval) -> i64 {
        1
    }

    #[php(returns = "int|string|null")]
    pub fn produce() -> i64 {
        0
    }

    #[php(returns = "\\PhpTypesAttrFoo|\\PhpTypesAttrBar")]
    pub fn produce_class_union() -> i64 {
        0
    }
}

#[cfg(php83)]
#[php_class]
pub struct PhpTypesAttrHolder83;

#[cfg(php83)]
#[php_impl]
impl PhpTypesAttrHolder83 {
    pub fn __construct() -> Self {
        Self
    }

    #[php(returns = "\\Countable&\\Traversable")]
    pub fn produce_intersection() -> i64 {
        0
    }

    #[php(returns = "(\\Countable&\\Traversable)|\\PhpTypesAttrFoo")]
    pub fn produce_dnf() -> i64 {
        0
    }
}

#[php_function]
pub fn test_attr_int_or_string(#[php(types = "int|string")] _value: &Zval) -> i64 {
    1
}

#[php_function]
#[php(returns = "int|string|null")]
pub fn test_attr_returns_int_string_or_null() -> i64 {
    0
}

#[php_function]
pub fn test_attr_class_union(
    #[php(types = "\\PhpTypesAttrFoo|\\PhpTypesAttrBar")] _value: &Zval,
) -> i64 {
    1
}

#[cfg(php83)]
#[php_function]
pub fn test_attr_intersection(#[php(types = "\\Countable&\\Traversable")] _value: &Zval) -> i64 {
    1
}

#[cfg(php83)]
#[php_function]
pub fn test_attr_dnf(
    #[php(types = "(\\Countable&\\Traversable)|\\PhpTypesAttrFoo")] _value: &Zval,
) -> i64 {
    1
}

#[php_function]
#[php(returns = "\\PhpTypesAttrFoo|\\PhpTypesAttrBar")]
pub fn test_attr_returns_class_union() -> i64 {
    0
}

#[cfg(php83)]
#[php_function]
#[php(returns = "\\Countable&\\Traversable")]
pub fn test_attr_returns_intersection() -> i64 {
    0
}

#[cfg(php83)]
#[php_function]
#[php(returns = "(\\Countable&\\Traversable)|\\PhpTypesAttrFoo")]
pub fn test_attr_returns_dnf() -> i64 {
    0
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    let builder = builder
        .class::<PhpTypesAttrFoo>()
        .class::<PhpTypesAttrBar>()
        .class::<PhpTypesAttrHolder>()
        .function(wrap_function!(test_attr_int_or_string))
        .function(wrap_function!(test_attr_returns_int_string_or_null))
        .function(wrap_function!(test_attr_class_union))
        .function(wrap_function!(test_attr_returns_class_union));

    #[cfg(php83)]
    let builder = builder
        .class::<PhpTypesAttrHolder83>()
        .function(wrap_function!(test_attr_intersection))
        .function(wrap_function!(test_attr_dnf))
        .function(wrap_function!(test_attr_returns_intersection))
        .function(wrap_function!(test_attr_returns_dnf));

    builder
}

#[cfg(test)]
mod tests {
    #[test]
    fn attr_int_or_string_metadata_matches_reflection() {
        assert!(crate::integration::test::run_php(
            "php_types_attr/php_types_attr.php"
        ));
    }
}
