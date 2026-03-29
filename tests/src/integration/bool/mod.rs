use ext_php_rs::prelude::*;

#[php_function]
pub fn test_bool(a: bool) -> bool {
    a
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder.function(wrap_function!(test_bool))
}

#[cfg(test)]
mod tests {
    #[test]
    fn bool_works() {
        assert!(crate::integration::test::run_php("bool/bool.php"));
    }

    #[test]
    #[cfg(feature = "embed")]
    fn bool_works_embed() {
        crate::integration::test::run_php_embed("bool/bool.php");
    }
}
