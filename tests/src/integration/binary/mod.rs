use ext_php_rs::{binary::Binary, prelude::*};

#[php_function]
pub fn test_binary(a: Binary<u32>) -> Binary<u32> {
    a
}

// Regression coverage for #729: packed length 0 or 1 returns an interned
// permanent static from ext_php_rs_zend_string_init; prior to the fix,
// Zval::set_binary flagged these as refcounted and caused heap corruption
// on drop.
#[php_function]
#[php(name = "test_binary_empty_u8")]
pub fn test_binary_empty_u8() -> Binary<u8> {
    Binary::from(Vec::<u8>::new())
}

#[php_function]
#[php(name = "test_binary_single_u8")]
pub fn test_binary_single_u8() -> Binary<u8> {
    Binary::from(vec![42u8])
}

#[php_function]
#[php(name = "test_binary_empty_u32")]
pub fn test_binary_empty_u32() -> Binary<u32> {
    Binary::from(Vec::<u32>::new())
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_binary))
        .function(wrap_function!(test_binary_empty_u8))
        .function(wrap_function!(test_binary_single_u8))
        .function(wrap_function!(test_binary_empty_u32))
}

#[cfg(test)]
mod tests {
    #[test]
    fn binary_works() {
        assert!(crate::integration::test::run_php("binary/binary.php"));
    }
}
