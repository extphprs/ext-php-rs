use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

#[php_function]
pub fn test_ref_string(ref_: &str) -> String {
    ref_.to_string()
}

#[php_function]
pub fn test_ref_long(ref_: i64) -> i64 {
    ref_
}

#[php_function]
pub fn test_ref_double(ref_: f64) -> f64 {
    ref_
}

/// Test accepting a mutable Zval reference - can modify the PHP variable in place.
#[php_function]
pub fn test_mut_zval(val: &mut Zval) -> String {
    // Return the original type as string
    if let Some(s) = val.str() {
        format!("string: {s}")
    } else if let Some(l) = val.long() {
        format!("long: {l}")
    } else {
        "unknown".to_string()
    }
}

/// Test modifying a Zval to change the PHP variable
#[php_function]
pub fn test_mut_zval_set_string(val: &mut Zval) {
    val.set_string("modified by rust", false).ok();
}

/// Test modifying via &mut Zval - set to a new string value
#[php_function]
pub fn test_mut_zval_set_long(val: &mut Zval) {
    val.set_long(999);
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_ref_string))
        .function(wrap_function!(test_ref_long))
        .function(wrap_function!(test_ref_double))
        .function(wrap_function!(test_mut_zval))
        .function(wrap_function!(test_mut_zval_set_string))
        .function(wrap_function!(test_mut_zval_set_long))
}

#[cfg(test)]
mod tests {
    #[test]
    fn reference_works() {
        assert!(crate::integration::test::run_php("reference/reference.php"));
    }
}
