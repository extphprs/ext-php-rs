use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

#[php_function]
pub fn test_persistent_string() -> String {
    let mut z = Zval::new();
    let _ = z.set_string("PERSISTENT_STRING", true);
    // z is dropped here - this was causing heap corruption before the fix
    "persistent string test passed".to_string()
}

#[php_function]
pub fn test_non_persistent_string() -> String {
    let mut z = Zval::new();
    let _ = z.set_string("NON_PERSISTENT_STRING", false);
    "non-persistent string test passed".to_string()
}

#[php_function]
pub fn test_persistent_string_read() -> String {
    let mut z = Zval::new();
    let _ = z.set_string("READ_BEFORE_DROP", true);
    let value = z.str().unwrap_or("failed");
    format!("read: {value}")
}

#[php_function]
pub fn test_persistent_string_loop(count: i64) -> String {
    for i in 0..count {
        let mut z = Zval::new();
        let s = format!("LOOP_{i}");
        let _ = z.set_string(&s, true);
    }
    format!("completed {count} iterations")
}

#[php_function]
pub fn test_interned_string_persistent() -> String {
    let mut z = Zval::new();
    let _ = z.set_interned_string("INTERNED_PERSISTENT", true);
    "interned persistent test passed".to_string()
}

#[php_function]
pub fn test_interned_string_non_persistent() -> String {
    let mut z = Zval::new();
    let _ = z.set_interned_string("INTERNED_NON_PERSISTENT", false);
    "interned non-persistent test passed".to_string()
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_persistent_string))
        .function(wrap_function!(test_non_persistent_string))
        .function(wrap_function!(test_persistent_string_read))
        .function(wrap_function!(test_persistent_string_loop))
        .function(wrap_function!(test_interned_string_persistent))
        .function(wrap_function!(test_interned_string_non_persistent))
}

#[cfg(test)]
mod tests {
    #[test]
    fn persistent_string_works() {
        assert!(crate::integration::test::run_php(
            "persistent_string/persistent_string.php"
        ));
    }
}
