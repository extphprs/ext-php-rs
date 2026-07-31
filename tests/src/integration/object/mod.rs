use ext_php_rs::{prelude::*, types::ZendObject};

#[php_function]
pub fn test_object(a: &mut ZendObject) -> &mut ZendObject {
    a
}

#[php_function]
pub fn test_object_to_string(a: &mut ZendObject) -> PhpResult<String> {
    a.extract::<String>().map_err(PhpException::from)
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_object))
        .function(wrap_function!(test_object_to_string))
}

#[cfg(test)]
mod tests {
    #[test]
    fn object_works() {
        assert!(crate::integration::test::run_php("object/object.php"));
    }

    #[test]
    fn object_to_string_works() {
        assert!(crate::integration::test::run_php("object/to_string.php"));
    }
}
