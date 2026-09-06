use ext_php_rs::{binary_slice::BinarySlice, prelude::*};

#[php_function]
pub fn test_binary_slice_sum(values: BinarySlice<u64>) -> u64 {
    values.iter().sum()
}

#[php_function]
pub fn test_binary_slice_len(values: BinarySlice<u64>) -> usize {
    values.len()
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_binary_slice_sum))
        .function(wrap_function!(test_binary_slice_len))
}

#[cfg(test)]
mod tests {
    #[test]
    fn binary_slice_works() {
        assert!(crate::integration::test::run_php(
            "binary_slice/binary_slice.php"
        ));
    }
}
