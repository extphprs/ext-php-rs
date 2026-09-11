use ext_php_rs::{
    enum_::RegisteredEnum,
    error::Result,
    exception::PhpException,
    php_enum, php_function,
    prelude::{ModuleBuilder, PhpResult},
    wrap_function,
};

#[php_enum]
#[php(allow_native_discriminants)]
/// An example enum that demonstrates how to use PHP enums with Rust.
/// This enum has two variants, `Variant1` and `Variant2`.
pub enum TestEnum {
    /// Represents the first variant of the enum.
    /// This variant has a discriminant of 0.
    /// But PHP does not know about it.
    Variant1,
    /// Represents the second variant of the enum.
    Variant2 = 1,
}

#[php_enum]
pub enum IntBackedEnum {
    #[php(value = 1)]
    Variant1,
    #[php(value = 2)]
    Variant2,
}

#[php_enum]
pub enum StringBackedEnum {
    #[php(value = "foo")]
    Variant1,
    #[php(value = "bar")]
    Variant2,
}

#[php_function]
pub fn test_enum(a: TestEnum) -> Result<StringBackedEnum> {
    let str: &str = StringBackedEnum::Variant2.into();
    match a {
        TestEnum::Variant1 => str.try_into(),
        TestEnum::Variant2 => Ok(StringBackedEnum::Variant1),
    }
}

#[php_function]
pub fn test_enum_from_name(name: String) -> PhpResult<StringBackedEnum> {
    StringBackedEnum::from_name(&name).map_err(PhpException::from)
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .enumeration::<TestEnum>()
        .enumeration::<IntBackedEnum>()
        .enumeration::<StringBackedEnum>()
        .function(wrap_function!(test_enum))
        .function(wrap_function!(test_enum_from_name))
}

#[cfg(test)]
mod tests {
    #[test]
    fn enum_works() {
        assert!(crate::integration::test::run_php("enum_/enum.php"));
    }
}
