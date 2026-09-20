use ext_php_rs::prelude::*;

#[php_class]
pub struct TestConstantVisibility;

#[php_impl]
impl TestConstantVisibility {
    const PUBLIC_CONST: i64 = 1;
    #[php(vis = "public")]
    const EXPLICIT_PUBLIC_CONST: i64 = 2;
    #[php(vis = "protected")]
    const PROTECTED_CONST: &'static str = "protected";
    #[php(vis = "private")]
    const PRIVATE_CONST: bool = true;

    pub fn read_private() -> bool {
        Self::PRIVATE_CONST
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder.class::<TestConstantVisibility>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn constant_visibility_works() {
        assert!(crate::integration::test::run_php("constant/constant.php"));
    }
}
