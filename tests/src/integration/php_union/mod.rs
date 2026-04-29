use ext_php_rs::prelude::*;

#[derive(PhpUnion)]
pub enum IntOrString {
    Int(i64),
    Str(String),
}

#[php_function]
pub fn test_php_union_param(value: IntOrString) -> i64 {
    match value {
        IntOrString::Int(_) => 1,
        IntOrString::Str(_) => 2,
    }
}

#[php_function]
pub fn test_php_union_return(flag: bool) -> IntOrString {
    if flag {
        IntOrString::Int(7)
    } else {
        IntOrString::Str("hi".to_owned())
    }
}

#[php_class]
pub struct PhpUnionHolder;

#[php_impl]
impl PhpUnionHolder {
    pub fn __construct() -> Self {
        Self
    }

    pub fn accept(&self, value: IntOrString) -> IntOrString {
        value
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .class::<PhpUnionHolder>()
        .function(wrap_function!(test_php_union_param))
        .function(wrap_function!(test_php_union_return))
}

#[cfg(test)]
mod tests {
    use super::IntOrString;
    use ext_php_rs::convert::{FromZval, FromZvalMut, IntoZval};
    use ext_php_rs::flags::DataType;
    use ext_php_rs::types::{PhpType, PhpUnion};

    #[test]
    fn union_types_emits_long_then_string() {
        assert_eq!(
            <IntOrString as PhpUnion>::union_types(),
            PhpType::Union(vec![DataType::Long, DataType::String]),
        );
    }

    #[test]
    fn into_zval_php_type_delegates_to_union_types() {
        assert_eq!(
            <IntOrString as IntoZval>::php_type(),
            PhpType::Union(vec![DataType::Long, DataType::String]),
        );
    }

    #[test]
    fn from_zval_php_type_delegates_to_union_types() {
        assert_eq!(
            <IntOrString as FromZval>::php_type(),
            PhpType::Union(vec![DataType::Long, DataType::String]),
        );
    }

    #[test]
    fn from_zval_mut_php_type_forwards_through_blanket() {
        assert_eq!(
            <IntOrString as FromZvalMut>::php_type(),
            PhpType::Union(vec![DataType::Long, DataType::String]),
        );
    }

    #[test]
    fn default_php_type_wraps_simple_for_primitive() {
        assert_eq!(
            <i64 as IntoZval>::php_type(),
            PhpType::Simple(DataType::Long)
        );
        assert_eq!(
            <i64 as FromZval>::php_type(),
            PhpType::Simple(DataType::Long)
        );
        assert_eq!(
            <i64 as FromZvalMut>::php_type(),
            PhpType::Simple(DataType::Long)
        );
    }

    #[test]
    fn option_forwards_php_type_to_inner() {
        assert_eq!(
            <Option<i64> as IntoZval>::php_type(),
            PhpType::Simple(DataType::Long),
        );
        assert_eq!(
            <Option<IntOrString> as IntoZval>::php_type(),
            PhpType::Union(vec![DataType::Long, DataType::String]),
        );
    }

    #[test]
    fn php_union_reflection_and_call_round_trip() {
        assert!(crate::integration::test::run_php("php_union/php_union.php"));
    }
}
